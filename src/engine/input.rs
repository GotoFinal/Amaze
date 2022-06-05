use std::collections::HashMap;
use std::ops::{Div, Mul};

use glam::{UVec2, Vec2, Vec3, Vec3Swizzles};
use VirtualKeyCode::{LControl, LShift, RControl, Space};
use winit::event::{Event, VirtualKeyCode};
use winit::event::VirtualKeyCode::{A, C, D, Down, Left, Right, RShift, S, Up, W, E, Q, Numpad4, Numpad6, Numpad8, Numpad2, Numpad9, Numpad3, PageUp, PageDown};
use winit_input_helper::WinitInputHelper;

pub type InputId = u32;

// TODO: reseign this garbage, its bad, but idk how to do it in rust.
// The basic idea:
// Action based input system, you register action like "move" or "shoot" or "mic",
// each action have it input type, for example "move" is 2d axis, "shoot"/"mic" is Bool
// Then each action can have multiple input providers, like moving can be WSAD keys or maybe mouse movement or analog joystick.
// There should be possibility to also change some inputs from being simple Bool to a toggle.
// Because single action can be implemented using both simple keys and analog input,
// the system must support wrapping some inputs into another, like AxisInput::from_buttons(positive, negative)
// Additionally the Input classes should be easy so serialise, as they would be later loaded from configuration files or other sources.
// System should also support feeding events to inputs/actions to support more complicated combos that require buffering,
// like double clicking or more complicated combinations and toggles that need to track their state internally.

// It should be possible to normalize inputs in future, so when adding diff controllers support it should return same values for same inputs,
// like avoiding one device returning analog value from 0 to 1 and other device from -1 to 1

// Actions must be also categorised based on their, so they can be disabled and enabled in groups based on current engine state.
// For example depending if player is running or swimming we might have multiple actions with SPACE input that either jumps or ascends,
// so it should be possible to enable/disable contexts, like:
// system.add_context("walk");
// system.swap_context("walk", "swimming")

// Code should be focus on being easy to use when using it outside of the module itself, so actually using actions and creating inputs.
// pseudo-code api usage:
// let action = system.new_action<Axis2D>("move", vec![Contexts::Walk, Contexts::Swim]);
// let input = Input2D::from_axis(
//                 InputAxis::from_buttons(InputKey::simple(vec![Up, W])), InputAxis::from_buttons(InputKey::simple(vec![Down, S])),  // Up OR W, etc....
//                 InputAxis::from_buttons(InputKey::simple(vec![Right, D])), InputAxis::from_buttons(InputKey::simple(vec![Left, A])),
//             );
// system.add_input(action, input);
// system.add_input(action, other_input);
// then anywhere in code: let move_input : Axis2D = action.get();
// or
// let action = system.new_action<Axis2D>("camera_rot", vec![Contexts::Walk, Contexts::Swim]);
// let input = Input2D::mouse_move();
// system.add_input(action, input);
// then anywhere in code: let camera_move : Axis2D = action.get();
// or
// let action = system.new_action<Bool>("mic", vec![Contexts::Global]);
// let input = InputButton::combo(vec![Ctrl, M]).as_toggle(); // Ctrl and M
// system.add_input(action, input);
// then anywhere in code: if(action.has_changed()) // where has_changed would be a flag set to true for single cpu frame when value changed.

// extra goals:
// Event support in actions, it would be great to be able to register simple functions as event handlers for certain inputs, like:
// let action = system.new_action<Bool>("pause");
// let input = InputButton::simple(vec![Esc]).as_toggle();
// system.add_input(action, input);
// TODO: how do i ensure access to engine state inside listener without polluting input code with engine code?
// action.on_change(on_pause_change);
// fn on_pause_change(&self /* engine state? */, action: Arc<dyn Action<Bool>>) {
//     self.paused = action.get();
// }
// But do i really need it? its all in the loop anyways. Can't even find good example.
//

pub const MOVE: InputId = 2;
pub const ASCEND: InputId = 5;
pub const ROTATE: InputId = 6;
pub const ACTION: InputId = 3;
pub const SECONDARY_ACTION: InputId = 4;

pub trait Input {
    fn create() -> Self;

    fn get_move(&self) -> Vec2;

    fn get_axis(&self, id: InputId) -> f32;

    fn get_axis2d(&self, id: InputId) -> Vec2;
    fn get_axis3d(&self, id: InputId) -> Vec3;

    fn get_mouse_move(&self) -> Vec2;

    fn get_mouse_position(&self) -> Vec2;

    fn get_mouse_position_normalized(&self) -> Vec2;

    fn is_action(&self) -> bool;

    fn send_event<'a, T>(&mut self, event: &Event<'a, T>);
}

pub struct InputSystem {
    pub system: WinitInputHelper,
    inputs: HashMap<InputId, Box<dyn ValuedInput>>,
}

// TODO: does not handle collisions between shortcuts, if 2 bindings are possible only the more advanced one should be activated
// TODO: need a way to go back from ValuedInput to raw type
impl Input for InputSystem {
    fn create() -> Self {
        let mut input = WinitInputHelper::new();
        let mut mapping: HashMap<InputId, Box<dyn ValuedInput>> = HashMap::new();
        mapping.insert(ACTION, Box::new(ButtonInput::simple(vec![LShift, RShift], 1.0)));
        mapping.insert(SECONDARY_ACTION, Box::new(ButtonInput::simple(vec![LControl, RControl], 1.0)));
        mapping.insert(MOVE, Box::new(
            PlaneInput {
                vertical: AxisInput {
                    positive: ButtonInput::simple(vec![Up, W], 1.0),
                    negative: ButtonInput::simple(vec![Down, S], -1.0),
                },
                horizontal: AxisInput {
                    positive: ButtonInput::simple(vec![Right, D], 1.0),
                    negative: ButtonInput::simple(vec![Left, A], -1.0),
                },
            }
        ));
        mapping.insert(ASCEND, Box::new(
            AxisInput {
                positive: ButtonInput::simple(vec![E], 1.0),
                negative: ButtonInput::simple(vec![Q], -1.0),
            }
        ));

        mapping.insert(ROTATE, Box::new(
            Axis3DInput {
                x: AxisInput {
                    positive: ButtonInput::simple(vec![Numpad4], 1.0),
                    negative: ButtonInput::simple(vec![Numpad6], -1.0),
                },
                y: AxisInput {
                    positive: ButtonInput::simple(vec![Numpad8], 1.0),
                    negative: ButtonInput::simple(vec![Numpad2], -1.0),
                },
                z: AxisInput {
                    positive: ButtonInput::simple(vec![Numpad9, PageUp], 1.0),
                    negative: ButtonInput::simple(vec![Numpad3, PageDown], -1.0),
                },
            }
        ));
        return InputSystem {
            system: input,
            inputs: mapping,
        };
    }

    fn get_move(&self) -> Vec2 {
        return self.inputs.get(&MOVE).unwrap()
            .as_vec2(self);
    }

    fn get_axis(&self, id: InputId) -> f32 {
        return self.inputs.get(&id).unwrap()
            .as_value(self);
    }

    fn get_axis2d(&self, id: InputId) -> Vec2 {
        return self.inputs.get(&id).unwrap()
            .as_vec2(self);
    }

    fn get_axis3d(&self, id: InputId) -> Vec3 {
        return self.inputs.get(&id).unwrap()
            .as_vec3(self);
    }

    fn get_mouse_move(&self) -> Vec2 {
        return self.system.mouse_diff().into();
    }

    fn get_mouse_position(&self) -> Vec2 {
        return self.system.mouse().unwrap_or_default().into();
    }

    fn get_mouse_position_normalized(&self) -> Vec2 {
        let pos = self.get_mouse_position();
        let resolution = self.system.resolution().unwrap_or((100, 100));
        let rel = pos.div(Vec2::new(resolution.0 as f32, resolution.1 as f32)) * 2.0;
        return rel - 1.0;
    }

    fn is_action(&self) -> bool {
        return self.inputs.get(&ACTION).unwrap()
            .as_bool(self);
    }

    fn send_event<'a, T>(&mut self, winit_event: &Event<'a, T>) {
        self.system.update(winit_event);
    }
}

struct KeyCombo {
    keys: Vec<VirtualKeyCode>,
    modifier: f32,
}

impl KeyCombo {
    fn simple(key: VirtualKeyCode) -> KeyCombo {
        return Self::simple_valued(key, 1.0);
    }
    fn simple_valued(key: VirtualKeyCode, value: f32) -> KeyCombo {
        return KeyCombo {
            keys: vec![key],
            modifier: value,
        };
    }
    fn double(key_a: VirtualKeyCode, key_b: VirtualKeyCode) -> KeyCombo {
        return Self::double_valued(key_a, key_b, 1.0);
    }
    fn double_valued(key_a: VirtualKeyCode, key_b: VirtualKeyCode, value: f32) -> KeyCombo {
        return KeyCombo {
            keys: vec![key_a, key_b],
            modifier: value,
        };
    }

    fn is_pressed(&self, system: &InputSystem) -> bool {
        for key in self.keys.as_slice() {
            if !system.key_pressed(key.clone()) {
                return false;
            }
        }
        return true;
    }
    fn is_held(&self, system: &InputSystem) -> bool {
        for key in self.keys.as_slice() {
            if !system.key_held(key.clone()) {
                return false;
            }
        }
        return true;
    }
    fn is_released(&self, system: &InputSystem) -> bool {
        for key in self.keys.as_slice() {
            if !system.key_released(key.clone()) {
                return false;
            }
        }
        return true;
    }
}

struct ButtonInput {
    combinations: Vec<KeyCombo>,
}

impl ButtonInput {
    fn simple(inputs: Vec<VirtualKeyCode>, value: f32) -> ButtonInput {
        let mut combinations = Vec::with_capacity(inputs.len());
        for input in inputs {
            combinations.push(KeyCombo::simple_valued(input, value));
        }
        return ButtonInput { combinations };
    }

    fn is_pressed(&self, system: &InputSystem) -> bool {
        for combination in self.combinations.as_slice() {
            if combination.is_pressed(system) {
                return true;
            }
        }
        return false;
    }
    fn is_held(&self, system: &InputSystem) -> bool {
        for combination in self.combinations.as_slice() {
            if combination.is_held(system) {
                return true;
            }
        }
        return false;
    }
    fn is_released(&self, system: &InputSystem) -> bool {
        for combination in self.combinations.as_slice() {
            if combination.is_released(system) {
                return true;
            }
        }
        return false;
    }
    fn get_value(&self, system: &InputSystem) -> f32 {
        for combination in self.combinations.as_slice() {
            if combination.is_held(system) {
                return combination.modifier;
            }
        }
        return 0.0;
    }
}

struct AxisInput {
    positive: ButtonInput,
    negative: ButtonInput,
}

impl AxisInput {
    fn new(positive: ButtonInput, negative: ButtonInput) -> AxisInput {
        return AxisInput { positive, negative };
    }

    fn get_value(&self, system: &InputSystem) -> f32 {
        let mut result = 0.0;
        result += self.positive.get_value(system);
        result += self.negative.get_value(system);
        return result;
    }
}

struct PlaneInput {
    horizontal: AxisInput,
    vertical: AxisInput,
}

struct Axis3DInput {
    x: AxisInput,
    y: AxisInput,
    z: AxisInput,
}

impl PlaneInput {
    fn new(horizontal: AxisInput, vertical: AxisInput) -> PlaneInput {
        return PlaneInput { horizontal, vertical };
    }

    fn get_value(&self, system: &InputSystem) -> Vec2 {
        let mut result = Vec2::ZERO;
        result.x += self.horizontal.get_value(system);
        result.y += self.vertical.get_value(system);
        return result.normalize_or_zero();
    }
}

impl Axis3DInput {
    fn new(x: AxisInput, y: AxisInput, z: AxisInput) -> Axis3DInput {
        return Axis3DInput { x, y, z };
    }

    fn get_value(&self, system: &InputSystem) -> Vec3 {
        let mut result = Vec3::ZERO;
        result.x += self.x.get_value(system);
        result.y += self.y.get_value(system);
        result.z += self.z.get_value(system);
        return result.normalize_or_zero();
    }
}

trait ValuedInput {
    fn as_bool(&self, system: &InputSystem) -> bool;
    fn as_value(&self, system: &InputSystem) -> f32;
    fn as_vec2(&self, system: &InputSystem) -> Vec2;
    fn as_vec3(&self, system: &InputSystem) -> Vec3;
}

impl ValuedInput for ButtonInput {
    fn as_bool(&self, system: &InputSystem) -> bool {
        return self.is_held(system);
    }

    fn as_value(&self, system: &InputSystem) -> f32 {
        return self.get_value(system);
    }

    fn as_vec2(&self, system: &InputSystem) -> Vec2 {
        let value = self.get_value(system);
        return Vec2::new(value, value).normalize_or_zero();
    }
    fn as_vec3(&self, system: &InputSystem) -> Vec3 {
        let value = self.get_value(system);
        return Vec3::new(value, value, value).normalize_or_zero();
    }
}

impl ValuedInput for AxisInput {
    fn as_bool(&self, system: &InputSystem) -> bool {
        return self.as_value(system) != 0.0;
    }

    fn as_value(&self, system: &InputSystem) -> f32 {
        return self.get_value(system);
    }

    fn as_vec2(&self, system: &InputSystem) -> Vec2 {
        let x = self.positive.get_value(system);
        let y = self.negative.get_value(system);
        return Vec2::new(x, y).normalize_or_zero();
    }
    fn as_vec3(&self, system: &InputSystem) -> Vec3 {
        let x = self.positive.get_value(system);
        let y = self.negative.get_value(system);
        let z = self.negative.get_value(system);
        return Vec3::new(x, y, z).normalize_or_zero();
    }
}

impl ValuedInput for PlaneInput {
    fn as_bool(&self, system: &InputSystem) -> bool {
        return self.as_value(system) != 0.0;
    }

    fn as_value(&self, system: &InputSystem) -> f32 {
        return self.get_value(system).length();
    }

    fn as_vec2(&self, system: &InputSystem) -> Vec2 {
        return self.get_value(system);
    }

    fn as_vec3(&self, system: &InputSystem) -> Vec3 {
        return self.get_value(system).extend(0.0);
    }
}

impl ValuedInput for Axis3DInput {
    fn as_bool(&self, system: &InputSystem) -> bool {
        return self.as_value(system) != 0.0;
    }

    fn as_value(&self, system: &InputSystem) -> f32 {
        return self.get_value(system).length();
    }

    fn as_vec2(&self, system: &InputSystem) -> Vec2 {
        return self.get_value(system).xy(); // so bad
    }
    fn as_vec3(&self, system: &InputSystem) -> Vec3 {
        return self.get_value(system);
    }
}

impl InputSystem {
    fn key_pressed(&self, key: VirtualKeyCode) -> bool {
        return self.system.key_pressed(key);
    }
    fn key_held(&self, key: VirtualKeyCode) -> bool {
        return self.system.key_held(key);
    }
    fn key_released(&self, key: VirtualKeyCode) -> bool {
        return self.system.key_released(key);
    }

    fn is_pressed(&self, input: &ButtonInput) -> bool {
        return input.is_pressed(self);
    }
    fn is_held(&self, input: &ButtonInput) -> bool {
        return input.is_held(self);
    }
    fn is_released(&self, input: &ButtonInput) -> bool {
        return input.is_released(self);
    }

    fn input(&self, input: &dyn ValuedInput) -> f32 {
        return input.as_value(self);
    }
    fn vec2(&self, input: &dyn ValuedInput) -> Vec2 {
        return input.as_vec2(self);
    }
}