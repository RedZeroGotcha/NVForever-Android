//! Android native input -> Bevy Gamepad -> the game's existing controller bindings.
use bevy::{
    input::{
        gamepad::{RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent, RawGamepadEvent},
        InputSystems,
    },
    prelude::*,
};
use nvf_android_input::Input;

#[derive(Resource, Clone, Copy)]
pub struct ControlsConfig {
    pub controller: bool,
    pub touch: bool,
    pub diagnostics: bool,
}
impl ControlsConfig {
    pub fn load() -> Self {
        let mut config = Self {
            controller: false,
            touch: true,
            diagnostics: false,
        };
        for line in crate::paths::option_file("nvforever-controls.txt")
            .unwrap_or_default()
            .lines()
        {
            let mut words = line.split_whitespace();
            let (Some(name), Some(value)) = (words.next(), words.next()) else {
                continue;
            };
            let enabled = match value {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => continue,
            };
            match name {
                "controller" => config.controller = enabled,
                "touch" => config.touch = enabled,
                "diagnostics" => config.diagnostics = enabled,
                _ => {}
            }
        }
        config
    }
}
#[derive(Resource)]
struct AndroidPad {
    entity: Entity,
    keys: [bool; 16],
    axes: [f32; 8],
}
const BUTTONS: [GamepadButton; 16] = {
    use GamepadButton::*;
    [
        South,
        East,
        West,
        North,
        LeftTrigger,
        RightTrigger,
        LeftTrigger2,
        RightTrigger2,
        LeftThumb,
        RightThumb,
        Start,
        Select,
        DPadUp,
        DPadDown,
        DPadLeft,
        DPadRight,
    ]
};
pub struct ControllerPlugin;
impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        let entity = app
            .world_mut()
            .spawn((
                Gamepad::default(),
                Name::new("Retroid / Android controller"),
            ))
            .id();
        app.insert_resource(AndroidPad {
            entity,
            keys: [false; 16],
            axes: [0.0; 8],
        })
        .add_systems(PreUpdate, pump.before(InputSystems))
        .add_systems(
            PostStartup,
            select_device.after(nvf_play::bindings::apply_stored),
        );
    }
}
/// M6 shipped these two directions reversed. Only migrate the complete legacy
/// four-way default, preserving any deliberately rebound controller profile.
fn migrate_legacy_dpad(bindings: &mut nvf_play::bindings::Bindings) -> bool {
    use nvf_play::{bindings::Control, pad::PadInput};
    let old = [
        (Control::Hotkey1, GamepadButton::DPadUp),
        (Control::AmmoSwap, GamepadButton::DPadRight),
        (Control::Hotkey3, GamepadButton::DPadDown),
        (Control::Hotkey4, GamepadButton::DPadLeft),
    ];
    if !old
        .iter()
        .all(|(control, button)| bindings.pad(*control) == Some(PadInput::Button(*button)))
    {
        return false;
    }
    bindings.set_pad(
        Control::AmmoSwap,
        Some(PadInput::Button(GamepadButton::DPadUp)),
    );
    bindings.set_pad(
        Control::Hotkey1,
        Some(PadInput::Button(GamepadButton::DPadRight)),
    );
    true
}

fn select_device(world: &mut World) {
    let on = world.resource::<ControlsConfig>().controller;
    if on {
        if let Some(mut bindings) = world.get_resource_mut::<nvf_play::bindings::Bindings>() {
            if migrate_legacy_dpad(&mut bindings) {
                info!("controller: migrated M6 defaults to console D-pad (Up changes ammo)");
            }
        }
    }
    nvf_play::bindings::set_device(
        world,
        if on {
            nvf_play::bindings::InputDevice::Controller
        } else {
            nvf_play::bindings::InputDevice::KeyboardMouse
        },
    );
}
fn pump(
    mut pad: ResMut<AndroidPad>,
    config: Res<ControlsConfig>,
    mut raw: MessageWriter<RawGamepadEvent>,
) {
    let events = nvf_android_input::drain();
    if !config.controller {
        return;
    }
    for event in events {
        if config.diagnostics {
            info!(?event, "Android controller input");
        }
        match event {
            Input::Button(index, down) => pad.keys[index] = down,
            Input::Axes(values) => pad.axes = values,
            Input::Reset => {
                pad.keys.fill(false);
                pad.axes.fill(0.0);
            }
        }
        for (index, button) in BUTTONS.iter().enumerate() {
            let digital = if pad.keys[index] { 1.0_f32 } else { 0.0 };
            let value = match index {
                6 => digital.max(pad.axes[4]),
                7 => digital.max(pad.axes[5]),
                12 => digital.max(if pad.axes[7] < -0.5 { 1.0 } else { 0.0 }),
                13 => digital.max(if pad.axes[7] > 0.5 { 1.0 } else { 0.0 }),
                14 => digital.max(if pad.axes[6] < -0.5 { 1.0 } else { 0.0 }),
                15 => digital.max(if pad.axes[6] > 0.5 { 1.0 } else { 0.0 }),
                _ => digital,
            };
            raw.write(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad.entity, *button, value,
            )));
        }
        for (index, axis) in [
            GamepadAxis::LeftStickX,
            GamepadAxis::LeftStickY,
            GamepadAxis::RightStickX,
            GamepadAxis::RightStickY,
        ]
        .into_iter()
        .enumerate()
        {
            raw.write(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                pad.entity,
                axis,
                pad.axes[index],
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvf_play::{
        bindings::{Bindings, Control},
        pad::PadInput,
    };

    #[test]
    fn legacy_migration_is_idempotent_and_preserves_custom_bindings() {
        let mut bindings = Bindings::game_defaults();
        assert!(!migrate_legacy_dpad(&mut bindings));
        bindings.set_pad(
            Control::Hotkey1,
            Some(PadInput::Button(GamepadButton::DPadUp)),
        );
        bindings.set_pad(
            Control::AmmoSwap,
            Some(PadInput::Button(GamepadButton::DPadRight)),
        );
        let mut custom = bindings.clone();
        custom.set_pad(
            Control::Hotkey4,
            Some(PadInput::Button(GamepadButton::North)),
        );
        assert!(!migrate_legacy_dpad(&mut custom));
        assert_eq!(
            custom.pad(Control::AmmoSwap),
            Some(PadInput::Button(GamepadButton::DPadRight))
        );
        assert!(migrate_legacy_dpad(&mut bindings));
        assert_eq!(
            bindings.pad(Control::AmmoSwap),
            Some(PadInput::Button(GamepadButton::DPadUp))
        );
        assert!(!migrate_legacy_dpad(&mut bindings));
    }

    #[test]
    fn console_actions_and_native_input_survive_frames_and_focus_loss() {
        use bevy::input::InputPlugin;
        use GamepadButton::*;
        nvf_android_input::drain();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(InputPlugin)
            .insert_resource(Bindings::game_defaults())
            .insert_resource(ControlsConfig {
                controller: true,
                touch: false,
                diagnostics: false,
            })
            .add_plugins(ControllerPlugin)
            .add_plugins(nvf_play::pad::PadPlugin)
            .insert_resource(State::new(nvf_play::PlayState::Menu));
        app.update();
        let entity = app.world().resource::<AndroidPad>().entity;
        let expected = [
            (Control::Activate, South),
            (Control::MenuMode, East),
            (Control::ReadyItem, West),
            (Control::Jump, North),
            (Control::Block, LeftTrigger2),
            (Control::Use, RightTrigger2),
            (Control::TogglePov, LeftTrigger),
            (Control::Vats, RightTrigger),
            (Control::CrouchSneak, LeftThumb),
            (Control::Grab, RightThumb),
            (Control::Rest, Select),
            (Control::AmmoSwap, DPadUp),
        ];
        for (control, button) in expected {
            assert_eq!(
                app.world().resource::<Bindings>().pad(control),
                Some(PadInput::Button(button))
            );
        }
        for (code, button) in [
            (96, South),
            (97, East),
            (99, West),
            (100, North),
            (102, LeftTrigger),
            (103, RightTrigger),
            (104, LeftTrigger2),
            (105, RightTrigger2),
            (106, LeftThumb),
            (107, RightThumb),
            (108, Start),
            (109, Select),
            (19, DPadUp),
            (20, DPadDown),
            (21, DPadLeft),
            (22, DPadRight),
        ] {
            assert!(nvf_android_input::key(code, true));
            app.update();
            assert!(app
                .world()
                .get::<Gamepad>(entity)
                .unwrap()
                .just_pressed(button));
            app.update();
            let pad = app.world().get::<Gamepad>(entity).unwrap();
            assert!(pad.pressed(button));
            assert!(!pad.just_pressed(button));
            assert!(nvf_android_input::key(code, false));
            app.update();
            assert!(app
                .world()
                .get::<Gamepad>(entity)
                .unwrap()
                .just_released(button));
        }
        nvf_android_input::push(Input::Axes([0.8, 0.6, -0.7, -0.5, 0.9, 1.0, -1.0, -1.0]));
        app.update();
        let pad = app.world().get::<Gamepad>(entity).unwrap();
        assert!(pad.left_stick().x > 0.5 && pad.left_stick().y > 0.3);
        assert!(pad.right_stick().x < -0.4 && pad.right_stick().y < -0.2);
        assert!(pad.pressed(LeftTrigger2) && pad.pressed(RightTrigger2));
        assert!(pad.pressed(DPadLeft) && pad.pressed(DPadUp));
        let keys = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(keys.pressed(KeyCode::ArrowUp) && keys.pressed(KeyCode::ArrowRight));
        nvf_android_input::push(Input::Reset);
        app.update();
        let pad = app.world().get::<Gamepad>(entity).unwrap();
        assert_eq!(pad.left_stick(), Vec2::ZERO);
        assert_eq!(pad.right_stick(), Vec2::ZERO);
        for button in BUTTONS {
            assert!(!pad.pressed(button));
        }
        assert!(!app
            .world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::ArrowUp));
        let mut pip = nvf_play::pipboy::PipBoy::default();
        pip.phase = nvf_play::pipboy::Phase::Up;
        pip.page = nvf_play::pipboy::Page::Stats;
        app.insert_resource(pip);
        nvf_android_input::key(104, true);
        app.update();
        assert!(app
            .world()
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(KeyCode::F3));
        app.update();
        assert!(!app
            .world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::F3));
        nvf_android_input::key(104, false);
        app.update();
        nvf_android_input::key(105, true);
        app.update();
        assert!(app
            .world()
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(KeyCode::F2));
        nvf_android_input::push(Input::Reset);
        app.update();
    }
}
