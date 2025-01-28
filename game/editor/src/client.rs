use std::{
    collections::VecDeque,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use anyhow::anyhow;
use base::system::{System, SystemTimeInterface};
use graphics::{
    graphics_mt::GraphicsMultiThreaded,
    handles::{
        backend::backend::GraphicsBackendHandle,
        buffer_object::buffer_object::GraphicsBufferObjectHandle,
        texture::texture::GraphicsTextureHandle,
    },
};
use math::math::vector::vec2;
use network::network::{event::NetworkEvent, types::NetworkClientCertCheckMode};
use sound::sound_mt::SoundMultiThreaded;

use crate::{
    action_logic::{redo_action, undo_action},
    actions::actions::{EditorAction, EditorActionGroup},
    event::{
        ActionDbg, AdminChangeConfig, AdminConfigState, ClientProps, EditorCommand, EditorEvent,
        EditorEventClientToServer, EditorEventGenerator, EditorEventOverwriteMap,
        EditorEventServerToClient, EditorNetEvent,
    },
    map::EditorMap,
    network::{EditorNetwork, NetworkState},
    notifications::{EditorNotification, EditorNotifications},
    tab::{EditorAdminPanel, EditorAdminPanelState},
};

/// the editor client handles events from the server if needed
pub struct EditorClient {
    network: EditorNetwork,

    has_events: Arc<AtomicBool>,
    event_generator: Arc<EditorEventGenerator>,

    notifications: EditorNotifications,
    local_client: bool,

    pub(crate) clients: Vec<ClientProps>,
    pub(crate) server_id: u64,
    pub(crate) allows_remote_admin: bool,

    pub(crate) msgs: VecDeque<(String, String)>,

    pub(crate) undo_label: Option<String>,
    pub(crate) redo_label: Option<String>,

    pub(crate) should_save: bool,

    last_keep_alive_id_and_time: (Option<u64>, Duration),
    sys: System,

    mapper_name: String,
    color: [u8; 3],
}

impl EditorClient {
    pub fn new(
        sys: &System,
        server_addr: &str,
        server_info: NetworkClientCertCheckMode,
        notifications: EditorNotifications,
        server_password: String,
        local_client: bool,
        mapper_name: Option<String>,
        color: Option<[u8; 3]>,
    ) -> Self {
        let has_events: Arc<AtomicBool> = Default::default();
        let event_generator = Arc::new(EditorEventGenerator::new(has_events.clone()));

        let res = Self {
            network: EditorNetwork::new_client(
                sys,
                event_generator.clone(),
                server_addr,
                server_info,
            ),
            has_events,
            event_generator,
            notifications,
            local_client,

            clients: Default::default(),
            server_id: Default::default(),
            allows_remote_admin: false,
            msgs: Default::default(),

            undo_label: None,
            redo_label: None,

            mapper_name: mapper_name.unwrap_or_else(|| "mapper".to_string()),
            color: color.unwrap_or([255, 255, 255]),

            last_keep_alive_id_and_time: (None, sys.time_get()),
            sys: sys.clone(),

            should_save: !local_client,
        };

        res.network
            .send(EditorEvent::Client(EditorEventClientToServer::Auth {
                password: server_password,
                is_local_client: local_client,
                mapper_name: res.mapper_name.clone(),
                color: res.color,
            }));

        res
    }

    pub fn net_state(&self) -> NetworkState {
        self.network.state()
    }

    pub fn update(
        &mut self,
        tp: &Arc<rayon::ThreadPool>,
        sound_mt: &SoundMultiThreaded,
        graphics_mt: &GraphicsMultiThreaded,
        buffer_object_handle: &GraphicsBufferObjectHandle,
        backend_handle: &GraphicsBackendHandle,
        texture_handle: &GraphicsTextureHandle,
        map: &mut EditorMap,
        admin_panel: &mut EditorAdminPanel,
    ) -> anyhow::Result<Option<EditorEventOverwriteMap>> {
        let mut res = None;

        if self.has_events.load(std::sync::atomic::Ordering::Relaxed) {
            let mut generated_events = self.event_generator.events.blocking_lock();

            let events = std::mem::take(&mut *generated_events);
            for (id, timestamp, event) in events {
                if res.is_some() {
                    generated_events.push_back((id, timestamp, event));
                    continue;
                }

                match event {
                    EditorNetEvent::Editor(EditorEvent::Server(ev)) => {
                        let undo_event = matches!(ev, EditorEventServerToClient::UndoAction { .. });
                        match ev {
                            EditorEventServerToClient::RedoAction {
                                action,
                                redo_label,
                                undo_label,
                            }
                            | EditorEventServerToClient::UndoAction {
                                action,
                                redo_label,
                                undo_label,
                            } => {
                                self.should_save = true;
                                if !self.local_client {
                                    let actions: Box<dyn Iterator<Item = _>> = if undo_event {
                                        Box::new(action.actions.into_iter().rev())
                                    } else {
                                        Box::new(action.actions.into_iter())
                                    };
                                    for act in actions {
                                        let act_func =
                                            if undo_event { undo_action } else { redo_action };
                                        if let Err(err) = act_func(
                                            tp,
                                            sound_mt,
                                            graphics_mt,
                                            buffer_object_handle,
                                            backend_handle,
                                            texture_handle,
                                            act,
                                            map,
                                        ) {
                                            self.notifications.push(EditorNotification::Error(
                                                format!(
                                                    "There has been a critical error while \
                                                    processing an action of the server: {err}.\n\
                                                    This usually indicates a bug in the \
                                                    editor code.\nCan not continue."
                                                ),
                                            ));
                                            return Err(anyhow!("critical error during do_action"));
                                        }
                                    }
                                }
                                self.undo_label = undo_label;
                                self.redo_label = redo_label;
                            }
                            EditorEventServerToClient::Error(err) => {
                                self.notifications.push(EditorNotification::Error(err));
                            }
                            EditorEventServerToClient::Map(map) => {
                                res = Some(map);
                            }
                            EditorEventServerToClient::Infos(infos) => {
                                self.clients = infos;
                            }
                            EditorEventServerToClient::Info {
                                server_id,
                                allows_remote_admin,
                            } => {
                                self.server_id = server_id;
                                self.allows_remote_admin = allows_remote_admin;
                            }
                            EditorEventServerToClient::Chat { from, msg } => {
                                self.notifications
                                    .push(EditorNotification::Info(format!("{from}: {msg}")));
                                self.msgs.push_front((from, msg));
                                self.msgs.truncate(30);
                            }
                            EditorEventServerToClient::AdminAuthed => {
                                admin_panel.state = match admin_panel.state.clone() {
                                    EditorAdminPanelState::NonAuthed(state) => {
                                        EditorAdminPanelState::Authed(AdminChangeConfig {
                                            password: state.password,
                                            state: AdminConfigState { auto_save: None },
                                        })
                                    }
                                    EditorAdminPanelState::Authed(state) => {
                                        EditorAdminPanelState::Authed(state)
                                    }
                                }
                            }
                            EditorEventServerToClient::AdminState { cur_state } => {
                                if let EditorAdminPanelState::Authed(state) = &mut admin_panel.state
                                {
                                    state.state = cur_state;
                                }
                            }
                        }
                    }

                    EditorNetEvent::Editor(EditorEvent::Client(_)) => {
                        // ignore
                    }
                    EditorNetEvent::NetworkEvent(ev) => {
                        if let NetworkEvent::NetworkStats(stats) = &ev {
                            if self
                                .last_keep_alive_id_and_time
                                .0
                                .is_none_or(|last_id| stats.last_keep_alive_id != last_id)
                                && timestamp >= self.last_keep_alive_id_and_time.1
                            {
                                self.last_keep_alive_id_and_time =
                                    (Some(stats.last_keep_alive_id), timestamp);
                            }
                        }

                        match self.network.handle_network_ev(id, ev) {
                            Ok(None) => {
                                // ignore
                            }
                            Ok(Some(msg)) => {
                                if !self.local_client {
                                    self.notifications.push(EditorNotification::Info(msg));
                                }
                            }
                            Err(err) => {
                                self.notifications
                                    .push(EditorNotification::Error(err.to_string()));
                            }
                        }
                    }
                }
            }
        }

        Ok(res)
    }

    pub fn execute(&self, action: EditorAction, group_identifier: Option<&str>) {
        self.network
            .send(EditorEvent::Client(EditorEventClientToServer::Action(
                EditorActionGroup {
                    actions: vec![action],
                    identifier: group_identifier.map(|s| s.to_string()),
                },
            )));
    }

    pub fn execute_group(&self, action_group: EditorActionGroup) {
        self.network
            .send(EditorEvent::Client(EditorEventClientToServer::Action(
                action_group,
            )));
    }

    pub fn undo(&self) {
        self.network
            .send(EditorEvent::Client(EditorEventClientToServer::Command(
                EditorCommand::Undo,
            )));
    }

    pub fn redo(&self) {
        self.network
            .send(EditorEvent::Client(EditorEventClientToServer::Command(
                EditorCommand::Redo,
            )));
    }

    pub fn update_info(&self, cursor_world_pos: vec2) {
        if !self.network.is_connected() {
            return;
        }

        self.network
            .send(EditorEvent::Client(EditorEventClientToServer::Info(
                ClientProps {
                    mapper_name: self.mapper_name.clone(),
                    color: self.color,
                    cursor_world: cursor_world_pos,
                    server_id: self.server_id,
                    stats: None,
                },
            )));
    }

    pub fn send_chat(&self, msg: String) {
        self.network
            .send(EditorEvent::Client(EditorEventClientToServer::Chat { msg }));
    }

    pub fn admin_auth(&self, password: String) {
        self.network
            .send(EditorEvent::Client(EditorEventClientToServer::AdminAuth {
                password,
            }));
    }

    pub fn admin_change_cfg(&self, state: AdminChangeConfig) {
        self.network.send(EditorEvent::Client(
            EditorEventClientToServer::AdminChangeConfig(state),
        ));
    }

    pub fn dbg_action(&self, props: ActionDbg) {
        self.network
            .send(EditorEvent::Client(EditorEventClientToServer::DbgAction(
                props,
            )));
    }

    /// Whether the connection to the server is most likely dead
    pub fn is_likely_distconnected(&self) -> bool {
        self.sys
            .time_get()
            .saturating_sub(self.last_keep_alive_id_and_time.1)
            > Duration::from_secs(6)
    }
}
