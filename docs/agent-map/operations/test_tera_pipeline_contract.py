"""Tera graph/text/cancel source assertions, NOT ORT/Rust/model behavior tests."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[3]


def source(path):return (ROOT/path).read_text()


class TeraPipelineSourceFixtures(unittest.TestCase):
    def test_runtime_marker_and_size_not_rehash_installer_owns_digest(self):
        runtime=source('suflyor-teratts/src/manifest.rs')
        body=runtime[runtime.index('pub fn check_installed'):runtime.index('pub fn installed_voices')]
        self.assertIn('metadata(',body);self.assertIn('meta.len()',body)
        self.assertNotIn('Sha256',body);self.assertNotIn('verify_file',body)
        installer=source('overlay-backend/src/teratts_install.rs')
        self.assertIn('pub fn verify_file',installer)
        self.assertIn('blob {}\\0',installer)

    def test_named_outputs_duration_validation_and_unbounded_initial_latent_alloc(self):
        s=source('suflyor-teratts/src/tera.rs')
        self.assertIn('sole_declared_output(',s)
        self.assertIn('named_output_f32(&encoder_outputs, &self.text_encoder_out)',s)
        self.assertIn('outputs.get(name)',s)
        self.assertIn('if !duration_seconds.is_finite() || duration_seconds <= 0.0',s)
        self.assertIn('let mut latent = vec![0.0_f32; LATENT_CHANNELS * latent_length]',s)
        self.assertLess(s.index('validate_latent_output(&latent_shape'),s.index('while start < latent_length'))
        self.assertIn('validate_vocoder_output(&wav_shape',s)

    def test_vocoder_overlap_returns_all_chunks_before_worker_event(self):
        tera=source('suflyor-teratts/src/tera.rs')
        self.assertIn('start.saturating_sub(VOCODER_CONTEXT_FRAMES)',tera)
        self.assertIn('chunks.push(chunk)',tera)
        self.assertIn('Ok(SynthOutput {',tera)
        main=source('suflyor-teratts/src/main.rs')
        worker=main[main.index('fn synth_worker('):main.index('fn worker(')]
        self.assertLess(worker.index('engine.synthesize('),worker.index('events.send('))
        self.assertIn('.map(|output| output.chunks)',worker)
        self.assertEqual(worker.count('generation.load('),1)
        self.assertNotIn('generation',tera)

    def test_text_pipeline_order_manual_stress_and_nested_span_unknowns(self):
        s=source('suflyor-teratts/src/textnorm.rs')
        body=s[s.index('pub fn prepare'):s.index('// Spacing passes')]
        tokens=['.nfc()','add_punctuation_spaces','add_number_word_spaces','skip_unsupported','validate_language_tags','expand_tagged_numbers','.nfkd()']
        positions=[body.index(t) for t in tokens];self.assertEqual(positions,sorted(positions))
        self.assertIn('model_text.replace(\'+\', "")',body)
        self.assertIn('stack.push(lang)',s)
        self.assertIn('spans.sort_by_key(|s| s.1)',s)
        self.assertIn('chars[cursor..content_start]',s)

    def test_npy_unchecked_shape_product_and_indexer_bmp_error(self):
        n=source('suflyor-teratts/src/npy.rs');i=source('suflyor-teratts/src/indexer.rs')
        self.assertIn('shape.iter().product()',n)
        self.assertIn('elements * 4',n)
        self.assertIn('Vec::with_capacity(elements)',n)
        self.assertIn('fortran-order arrays are not supported',n)
        self.assertIn('TABLE_LEN: usize = 65_536',i)
        self.assertIn('cp >= TABLE_LEN as u32',i)
        self.assertIn('unsupported character U+{:04X}',i)

    def test_chunk_limit_long_word_and_generation_stale_result_gates(self):
        c=source('suflyor-teratts/src/chunk.rs')
        self.assertIn('MAX_CHUNK_CHARS: usize = 120',c)
        hard=c[c.index('fn hard_split'):c.index('#[cfg(test)]')]
        self.assertIn('s.split_whitespace()',hard)
        self.assertIn('cur.push_str(word)',hard)
        self.assertNotIn('word.chars().take',hard)
        m=source('suflyor-teratts/src/main.rs')
        self.assertIn('self.generation.store(0, Ordering::Release)',m)
        self.assertIn('if outcome.utterance != active',m)
        self.assertIn('mpsc::channel::<SynthJob>()',m)
        self.assertIn('std::panic::catch_unwind',m)


if __name__=='__main__':unittest.main()
