use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    Device, FromSample, OutputCallbackInfo, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
};

use super::dsp::Frame;
use super::mixer::{Mixer, Mode};
use super::ring::{Writer, ring};

const BLOCK: usize = 1_024;
const COMMANDS: usize = 256;

pub struct Output {
    _stream: Stream,
}

#[must_use]
pub fn start(mode: Mode) -> Option<(Output, Writer)> {
    let device = cpal::default_host().default_output_device()?;
    let supported = device.default_output_config().ok()?;
    let format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let (writer, reader) = ring(COMMANDS);
    let mixer = Mixer::new(hertz(config.sample_rate), mode, reader);
    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, config, mixer),
        SampleFormat::I16 => build::<i16>(&device, config, mixer),
        SampleFormat::U16 => build::<u16>(&device, config, mixer),
        SampleFormat::I32 => build::<i32>(&device, config, mixer),
        _ => None,
    }?;
    stream.play().ok()?;
    Some((Output { _stream: stream }, writer))
}

fn build<T>(device: &Device, config: StreamConfig, mut mixer: Mixer) -> Option<Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let mut scratch = vec![[0.0; 2]; BLOCK];
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &OutputCallbackInfo| {
                fill(data, channels, &mut mixer, &mut scratch);
            },
            |error| eprintln!("audio: {error}"),
            None,
        )
        .ok()
}

pub fn fill<T: Sample + FromSample<f32>>(
    data: &mut [T],
    channels: usize,
    mixer: &mut Mixer,
    scratch: &mut [Frame],
) {
    let channels = channels.max(1);
    for chunk in data.chunks_mut(channels * scratch.len().max(1)) {
        let block = &mut scratch[..chunk.len() / channels];
        mixer.render(block);
        for (out, &[left, right]) in chunk.chunks_mut(channels).zip(block.iter()) {
            if let [only] = out {
                *only = T::from_sample(f32::midpoint(left, right));
                continue;
            }
            out[0] = T::from_sample(left);
            out[1] = T::from_sample(right);
            for extra in &mut out[2..] {
                *extra = T::from_sample(0.0);
            }
        }
    }
}

pub(crate) fn hertz(rate: u32) -> f32 {
    f32::from(u16::try_from(rate / 25).unwrap_or(u16::MAX)) * 25.0
}

#[cfg(test)]
mod tests {
    use super::fill;
    use crate::audio::dsp::Frame;
    use crate::audio::mixer::{Mixer, Mode};
    use crate::audio::ring::ring;

    const RATE: f32 = 48_000.0;

    fn mixer() -> Mixer {
        Mixer::new(RATE, Mode::Speakers, ring(8).1)
    }

    fn expected(frames: usize) -> Vec<Frame> {
        let mut out = vec![[0.0; 2]; frames];
        mixer().render(&mut out);
        out
    }

    #[test]
    fn stereo_devices_get_left_and_right_in_order() {
        let mut data = vec![0.0_f32; 2 * 3_000];
        fill(&mut data, 2, &mut mixer(), &mut vec![[0.0; 2]; 1_024]);
        for (pair, frame) in data.chunks(2).zip(expected(3_000)) {
            assert_eq!(pair, frame);
        }
    }

    #[test]
    fn extra_channels_stay_silent_and_mono_devices_get_the_average() {
        let mut wide = vec![1.0_f32; 6 * 500];
        fill(&mut wide, 6, &mut mixer(), &mut vec![[0.0; 2]; 256]);
        let mut mono = vec![1.0_f32; 500];
        fill(&mut mono, 1, &mut mixer(), &mut vec![[0.0; 2]; 256]);
        for ((six, one), [left, right]) in wide.chunks(6).zip(&mono).zip(expected(500)) {
            assert_eq!(&six[..2], &[left, right]);
            assert!(six[2..].iter().all(|&extra| extra == 0.0));
            assert!((one - f32::midpoint(left, right)).abs() < 1e-7);
        }
    }

    #[test]
    fn integer_devices_get_scaled_samples() {
        let mut data = vec![0_i16; 2 * 4_800];
        fill(&mut data, 2, &mut mixer(), &mut vec![[0.0; 2]; 1_024]);
        assert!(data.iter().any(|&sample| sample != 0));
        assert!(data.iter().all(|&sample| sample.unsigned_abs() < 8_000));
    }
}
