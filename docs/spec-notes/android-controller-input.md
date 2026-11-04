# Android controller input for the M6 release

## Public behaviour sources

- [documented] The Bethesda *Fallout: New Vegas* Xbox 360 manual, printed p. 2
  ([PDF mirror](https://device.report/m/df61014b29150201cb222cb61b0d2123320e13c39dd0793fc928e16b43b19419.pdf)),
  assigns A activation, B the Pip-Boy, X reload/ready, Y jump; LT aim/block,
  RT attack, LB view, RB V.A.T.S.; stick clicks sneak/grab, Back wait and Start
  pause. Holding B controls the light, holding X holsters, and holding LB adjusts
  the view. The Up direction changes ammunition; the remaining D-pad directions
  are for hotkeys. These are semantic Xbox button names, not an assumption about
  the letters printed on every Android device.
- [documented] The same manual, printed p. 6, describes LT/RT selecting the three
  major Pip-Boy sections and the left stick/D-pad navigating lists/tabs within a
  section. These are separate operations.
- [documented] Android's
  [KeyEvent constants](https://developer.android.com/reference/android/view/KeyEvent)
  define A/B/X/Y as 96/97/99/100; L1/R1/L2/R2 as 102–105; thumb clicks as 106/107;
  Start/Select as 108/109 and D-pad Up/Down/Left/Right as 19–22.
- [documented] Android's
  [controller input guide](https://developer.android.com/develop/ui/views/touch-and-input/game-controllers/controller-input)
  describes joystick motion with X/Y, Z/RZ, HAT_X/HAT_Y and trigger axes, with
  BRAKE/GAS available as alternatives for trigger values. Android stick Y is
  downward-positive; the adapter converts it for Bevy's upward-positive sticks.

## Device observations (2026-11-04)

- [observed] On a Retroid Pocket 5 running Android 13, recording `getevent -lt`
  while the owner pressed physical A, B, X, Y with pauses produced Linux
  BTN_GAMEPAD, BTN_EAST, BTN_WEST, BTN_NORTH in that order. Its installed
  `Vendor_2022_Product_3001.kl` maps those events to Android A, B, X, Y. This verifies
  that device's current layout, rather than all possible system ABXY settings.
- [observed] This RP5 exposes analog triggers through BRAKE/GAS. The adapter takes
  the larger value of each standard trigger and its fallback, so digital trigger
  events and motion axes can coexist without releasing each other.
- [observed] The ARM64 headless Bevy test executable was run through ADB on that
  RP5. All button mappings, cross-frame holds/releases, axes, trigger values,
  D-pad hats, menu stick directions, Pip-Boy section edges and reset passed.
  These are adapter observations, not measurements of the original game.

## Implementation decisions and boundaries

The native winit handler forwards controller keys and consumes joystick motion
before it can become touch input. A std-only queue preserves event order between
the native loop and Bevy. Focus loss queues a reset. The Bevy adapter updates an
ordinary Gamepad before InputSystems, keeping the existing gameplay bindings and
hold logic. It is opt-in; touch controls have a separate startup switch.

Only an entire unchanged legacy four-way D-pad default is migrated; an edited
profile remains the player's choice. Menu navigation shares the existing input
gate so rebind capture does not also navigate the menu. Pip-Boy LT/RT emits one
section-selection edge per trigger press, rather than repeatedly cycling while held.

This adapter targets the published M6 release. It does not establish full console
menu parity, all eight direct hotkey directions, V.A.T.S. stick-selection parity,
Android vibration, or exact original hold thresholds. Those require separate
implementation and original-game validation. No game executable, disassembly,
decompilation, leaked source or debug symbols were used.
