use std::collections::HashMap;

use glam::Vec2;
use VirtualKeyCode::{LControl, LShift, RControl, Space};
use winit::event::{Event, VirtualKeyCode};
use winit::event::VirtualKeyCode::{A, D, Down, Left, Right, RShift, S, Up, W};
use winit_input_helper::WinitInputHelper;

pub type InputId = u32;

pub trait Input {
    const MOVE: InputId = 2;
    const ACTION: InputId = 3;
    const SECONDARY_ACTION: InputId = 4;

    fn create() -> Self;

    fn get_move(&self) -> Vec2;

    fn is_action(&self) -> bool;

    fn send_event<'a, T>(&mut self, event: Event<'a, T>) -> Event<'a, T>;
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
        mapping.insert(Self::ACTION, Box::new(ButtonInput::simple(vec![Space, LShift, RShift], 1.0)));
        mapping.insert(Self::SECONDARY_ACTION, Box::new(ButtonInput::simple(vec![LControl, RControl], 1.0)));
        mapping.insert(Self::MOVE, Box::new(
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
        return InputSystem {
            system: input,
            inputs: mapping,
        };
    }

    fn get_move(&self) -> Vec2 {
        return self.inputs.get(&Self::MOVE).unwrap()
            .as_vec2(self)
    }

    fn is_action(&self) -> bool {
        return self.inputs.get(&Self::ACTION).unwrap()
            .as_bool(self)
    }

    fn send_event<'a, T>(&mut self, winit_event: Event<'a, T>) -> Event<'a, T> {
        self.system.update(&winit_event);
        winit_event
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

trait ValuedInput {
    fn as_bool(&self, system: &InputSystem) -> bool;
    fn as_value(&self, system: &InputSystem) -> f32;
    fn as_vec2(&self, system: &InputSystem) -> Vec2;
}

impl ValuedInput for ButtonInput {
    fn as_bool(&self, system: &InputSystem) -> bool {
        return self.is_held(system)
    }

    fn as_value(&self, system: &InputSystem) -> f32 {
        return self.get_value(system);
    }

    fn as_vec2(&self, system: &InputSystem) -> Vec2 {
        let value = self.get_value(system);
        return Vec2::new(value, value).normalize_or_zero();
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