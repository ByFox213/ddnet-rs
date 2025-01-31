#[derive(Default)]
pub struct WeaponSpec {
    // in ms
    pub fire_delay: u32,
    pub ammo_regen_time: u32,
    pub damage: u32,
    pub visual_size: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub muzzle_offset_x: f32,
    pub muzzle_offset_y: f32,
}

pub struct WeaponHammerSpec;
impl WeaponHammerSpec {
    pub fn get() -> WeaponSpec {
        WeaponSpec {
            fire_delay: 125,
            ammo_regen_time: 0,
            damage: 3,
            visual_size: 3.0,
            offset_x: 1.0 / 8.0,
            offset_y: -20.0 / 32.0,
            ..Default::default()
        }
    }
}

pub struct WeaponGunSpec;
impl WeaponGunSpec {
    pub fn get() -> WeaponSpec {
        WeaponSpec {
            fire_delay: 125,
            ammo_regen_time: 500,
            damage: 0,
            visual_size: 2.0,
            offset_x: 1.0,
            offset_y: 1.0 / 8.0,
            // the number after the plus sign is the sprite scale, which is calculated for all sprites ( w / sqrt(w² * h²) ) of the additionally added x offset, which is added now,
            // since the muzzle image is 32 pixels bigger, divided by 2, because a sprite's position is always at the center of the sprite image itself
            // => the offset added, bcs the sprite is bigger now, but should not be shifted to the left
            // => 96 / sqrt(64×64+96×96)  (the original sprite scale)
            // => 64 × original sprite scale (the actual size of the sprite ingame see weapon.visual_size above)
            // => (actual image sprite) / 3 (the new sprite is 128 instead of 96, so 4 / 3 times as big(bcs it should look the same as before, not scaled down because of this bigger number), so basically, 1 / 3 of the original size is added)
            // => (new sprite width) / 2 (bcs the sprite is at center, only add half of that new extra width)
            muzzle_offset_x: (50.0 + 8.8752) / 32.0,
            muzzle_offset_y: 6.0 / 32.0,
        }
    }
}

pub struct WeaponShotgunSpec;
impl WeaponShotgunSpec {
    pub fn get() -> WeaponSpec {
        WeaponSpec {
            fire_delay: 500,
            visual_size: 3.0,
            offset_x: 0.75,
            offset_y: -2.0 / 32.0,
            // see gun for the number after the plus sign
            muzzle_offset_x: (70.0 + 13.3128) / 32.0,
            muzzle_offset_y: 6.0 / 32.0,
            ..Default::default()
        }
    }
}

pub struct WeaponGrenadeSpec;
impl WeaponGrenadeSpec {
    pub fn get() -> WeaponSpec {
        WeaponSpec {
            fire_delay: 500,
            visual_size: 3.0,
            offset_x: 0.75,
            offset_y: -2.0 / 32.0,
            ..Default::default()
        }
    }
}

pub struct WeaponLaserSpec;
impl WeaponLaserSpec {
    pub fn get() -> WeaponSpec {
        WeaponSpec {
            fire_delay: 800,
            visual_size: 92.0 / 32.0,
            damage: 5,
            offset_x: 0.75,
            offset_y: -2.0 / 32.0,
            ..Default::default()
        }
    }
}

pub struct WeaponNinjaSpec;
impl WeaponNinjaSpec {
    pub fn get() -> WeaponSpec {
        WeaponSpec {
            fire_delay: 800,
            visual_size: 3.0,
            damage: 9,
            offset_x: 0.0,
            offset_y: 0.0,
            muzzle_offset_x: 40.0 / 32.0,
            muzzle_offset_y: -4.0 / 32.0,
            ..Default::default()
        }
    }
}
