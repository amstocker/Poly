mod engine;


fn main() {
    use crate::engine::Engine;

    let mut engine = Engine::default();

    let unit_state = engine.new_state();
    let unit_action = engine.new_action(unit_state);

    let a = engine.new_state();
    let b = engine.new_state();

    let atoa = engine.new_action(a);
    let atob = engine.new_action(a); 
    let btoa = engine.new_action(b); 
    let btob = engine.new_action(b);

    let comonad_a = engine.new_lens(a, a,
        &[
            (&[atoa, atoa], &[atoa]),
            (&[atoa, atob], &[atob]),
            (&[atob, btoa], &[atoa]),
            (&[atob, btob], &[atob]),
        ]
    );

    let comonad_b = engine.new_lens(b, b,
        &[
            (&[btoa, atoa], &[btoa]),
            (&[btoa, atob], &[btob]),
            (&[btob, btoa], &[btoa]),
            (&[btob, btob], &[btob]),
        ]
    );

    let counit_a = engine.new_lens(a, a, &[(&[unit_action], &[atoa])]);

    let counit_b = engine.new_lens(b, b, &[(&[unit_action], &[btob])]);
}
