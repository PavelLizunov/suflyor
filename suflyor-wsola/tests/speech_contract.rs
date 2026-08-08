#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use suflyor_wsola::Wsola;

const SAMPLE_RATE: usize = 16_000;
const SEGMENT: usize = SAMPLE_RATE * 30 / 1_000;
const SEARCH: usize = SAMPLE_RATE * 15 / 1_000;

fn speech_like_input() -> Vec<f32> {
    (0..SAMPLE_RATE * 3)
        .map(|sample| {
            let carrier =
                (sample as f32 * 180.0 * std::f32::consts::TAU / SAMPLE_RATE as f32).sin();
            let envelope = if sample % 2_000 < 1_700 { 0.55 } else { 0.08 };
            carrier * envelope
        })
        .collect()
}

#[test]
fn identity_is_deterministic_and_preserves_length() {
    let input = speech_like_input();
    let mut first = Wsola::new(SEGMENT, SEARCH, 1.0);
    let mut second = Wsola::new(SEGMENT, SEARCH, 1.0);
    let output = first.process(&input).unwrap();

    assert_eq!(output.len(), input.len());
    assert_eq!(output, second.process(&input).unwrap());
}

#[test]
fn speech_speed_ratios_stay_within_timeline_bounds() {
    let input = speech_like_input();
    for speed in [1.0_f64, 1.5, 2.0, 3.0] {
        let output = Wsola::new(SEGMENT, SEARCH, 1.0 / speed)
            .process(&input)
            .unwrap();
        let expected = input.len() as f64 / speed;
        let tolerance = SAMPLE_RATE as f64 / 25.0;
        assert!(
            ((output.len() as f64) - expected).abs() <= tolerance,
            "{speed}x emitted {} samples; expected {expected:.0} +/- {tolerance:.0}",
            output.len()
        );
        assert!(output.iter().all(|sample| sample.is_finite()));
    }
}
