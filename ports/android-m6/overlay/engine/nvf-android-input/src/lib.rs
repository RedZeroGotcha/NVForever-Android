//! Shared event queue between Android's native input loop and the gamepad plugin.
//! Keep transitions (rather than only the latest state) so short button taps survive.
use std::sync::Mutex;
#[derive(Clone, Copy, Debug)]
pub enum Input {
    Button(usize, bool),
    Axes([f32; 8]),
    Reset,
}
static EVENTS: Mutex<Vec<Input>> = Mutex::new(Vec::new());
pub fn push(event: Input) {
    EVENTS.lock().unwrap().push(event);
}
pub fn drain() -> Vec<Input> {
    std::mem::take(&mut *EVENTS.lock().unwrap())
}
pub fn key(code: u32, pressed: bool) -> bool {
    let button = match code {
        96 => 0,
        97 => 1,
        99 => 2,
        100 => 3, // A B X Y, Android's semantic layout
        102 => 4,
        103 => 5,
        104 => 6,
        105 => 7, // LB RB LT RT
        106 => 8,
        107 => 9,
        108 => 10,
        109 => 11, // LS RS Start Select
        19 => 12,
        20 => 13,
        21 => 14,
        22 => 15, // D-pad
        _ => return false,
    };
    push(Input::Button(button, pressed));
    true
}
/// Native Android axes use downward-positive Y and expose triggers on either
/// LTRIGGER/RTRIGGER or BRAKE/GAS (the latter on the RP5).
pub fn normalized_axes(values: [f32; 10]) -> [f32; 8] {
    let [x, y, z, rz, lt, rt, brake, gas, hat_x, hat_y] = values;
    [x, -y, z, -rz, lt.max(brake), rt.max(gas), hat_x, hat_y]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapping_preserves_taps_and_reset() {
        assert_eq!(
            normalized_axes([0.5, -0.5, -0.3, 0.3, 0.2, 0.8, 0.9, 0.1, -1.0, 1.0]),
            [0.5, 0.5, -0.3, -0.3, 0.9, 0.8, -1.0, 1.0]
        );
        drain();
        assert!(key(96, true));
        assert!(key(96, false));
        assert!(!key(24, true)); // volume must remain a system key
        push(Input::Reset);
        let events = drain();
        assert!(matches!(
            events.as_slice(),
            [
                Input::Button(0, true),
                Input::Button(0, false),
                Input::Reset
            ]
        ));
        assert!(drain().is_empty());
        for (index, code) in [
            96, 97, 99, 100, 102, 103, 104, 105, 106, 107, 108, 109, 19, 20, 21, 22,
        ]
        .into_iter()
        .enumerate()
        {
            assert!(key(code, true));
            assert!(matches!(drain().as_slice(), [Input::Button(i,true)] if *i == index));
        }
        push(Input::Axes([0.7, -0.5, 0.4, -0.3, 0.8, 0.9, 1.0, -1.0]));
        assert!(matches!(drain().as_slice(), [Input::Axes(a)] if a[0] == 0.7 && a[5] == 0.9));
    }
}
