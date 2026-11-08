//! Regression test for the tab close button.
//!
//! egui 0.37 resolves hover and clicks from the *previous* pass's widget rects.
//! A close button that only exists while the tab is hovered therefore steals the
//! hover from the tab, disappears, reappears... and its clicks land on the tab.
//! Registering it unconditionally is what makes the X work.

use eframe::egui::{self, pos2, vec2, Event, Id, PointerButton, Pos2, RawInput, Rect, Sense};

fn tab_rect() -> Rect {
    Rect::from_min_max(pos2(100.0, 10.0), pos2(220.0, 40.0))
}

fn close_rect() -> Rect {
    Rect::from_center_size(pos2(204.0, 25.0), vec2(18.0, 18.0))
}

fn tab_center() -> Pos2 {
    pos2(150.0, 25.0)
}

fn close_center() -> Pos2 {
    pos2(204.0, 25.0)
}

/// Run one frame. `always_close` = true models the fix (close always registered);
/// false models the old code (close only registered while the tab is hovered).
fn frame(
    ctx: &egui::Context,
    time: f64,
    pos: Pos2,
    button: Option<bool>,
    always_close: bool,
) -> (bool, bool) {
    let mut input = RawInput::default();
    input.time = Some(time);
    input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0)));
    input.events.push(Event::PointerMoved(pos));
    if let Some(pressed) = button {
        input.events.push(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        });
    }
    let mut close_clicked = false;
    let mut tab_clicked = false;
    let output = ctx.run_ui(input, |ui| {
        let tab = ui.interact(tab_rect(), Id::new("tab"), Sense::click());
        if always_close || tab.hovered() {
            let close = ui.interact(close_rect(), Id::new("tab-close"), Sense::click());
            close_clicked = close.clicked();
        }
        tab_clicked = tab.clicked();
    });
    output.drop_without_applying_deltas();
    (close_clicked, tab_clicked)
}

/// Move onto the tab, then onto its X, then press and release on the X.
fn click_close(always_close: bool) -> (bool, bool) {
    let ctx = egui::Context::default();
    let mut t = 0.0;
    let mut step = |pos: Pos2, button: Option<bool>| {
        t += 1.0 / 60.0;
        frame(&ctx, t, pos, button, always_close)
    };
    let _ = step(tab_center(), None); // warm-up (no hover state yet)
    let _ = step(tab_center(), None); // tab hovered
    let _ = step(close_center(), None); // pointer over the X
    let (_, tab_on_press) = step(close_center(), Some(true)); // press
    let (close_on_release, _) = step(close_center(), Some(false)); // release
    (close_on_release, tab_on_press)
}

#[test]
fn close_button_registered_on_every_pass_receives_the_click() {
    let (close_clicked, _) = click_close(true);
    assert!(
        close_clicked,
        "the X must receive the click when it is always registered"
    );
}

#[test]
fn hover_only_close_button_swallows_its_own_click() {
    let (close_clicked, _) = click_close(false);
    assert!(
        !close_clicked,
        "hover-only registration is the bug: the click resolves to the tab instead"
    );
}
