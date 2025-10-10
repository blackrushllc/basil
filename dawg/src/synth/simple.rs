use std::f32::consts::PI;

pub struct Voice { pub note: u8, pub phase: f32, pub inc: f32, pub vel: f32 }

pub struct Synth {
    pub sr: f32,
    pub voices: Vec<Voice>,
    pub max_poly: usize,
}

impl Synth {
    pub fn new(sr: f32, max_poly: usize) -> Self { Self { sr, voices: vec![], max_poly } }

    pub fn note_on(&mut self, note: u8, vel: u8) {
        let f = 440.0 * 2f32.powf((note as f32 - 69.0)/12.0);
        if self.voices.len() >= self.max_poly { self.voices.remove(0); }
        self.voices.push(Voice { note, phase: 0.0, inc: (2.0*PI*f)/self.sr, vel: vel as f32 / 127.0 });
    }
    pub fn note_off(&mut self, note: u8) { self.voices.retain(|v| v.note != note); }

    pub fn render_mono(&mut self, out: &mut [f32]) {
        for s in out.iter_mut() {
            let mut sum = 0.0;
            for v in &mut self.voices {
                sum += (v.phase).sin() * v.vel;
                v.phase += v.inc; if v.phase > 2.0*PI { v.phase -= 2.0*PI; }
            }
            *s = (sum * 0.2).clamp(-1.0, 1.0);
        }
    }
}
