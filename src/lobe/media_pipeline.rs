//! `MediaProcessor` — audio/video processing trait for multi-modal lobes.
//!
//! Temporal implements this trait to receive WebRTC audio+video RTP
//! packets, accumulate them, perform VAD-based segmentation, extract
//! video frames at ASR timestamps, and produce structured results.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrResult {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub speaker_id: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFrame {
    pub jpeg_base64: String,
    pub timestamp_ms: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VadState {
    Silence,
    Speaking { start_ms: u64 },
}

#[async_trait]
pub trait MediaProcessor: Send + Sync {
    async fn process_audio_rtp(&mut self, packet: &[u8], timestamp_ms: u64);

    async fn process_video_rtp(&mut self, packet: &[u8], timestamp_ms: u64);

    fn take_asr_result(&mut self) -> Option<AsrResult>;

    fn vad_state(&self) -> VadState;

    fn extract_frame(&self, timestamp_ms: u64) -> Option<VideoFrame>;

    fn recent_frames(&self, since_ms: u64) -> Vec<VideoFrame>;
}

pub fn empty_asr() -> AsrResult {
    AsrResult {
        text: String::new(),
        start_ms: 0,
        end_ms: 0,
        speaker_id: None,
        confidence: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asr_result_serializes() {
        let r = AsrResult {
            text: "hello".into(),
            start_ms: 100,
            end_ms: 200,
            speaker_id: Some("spk1".into()),
            confidence: 0.95,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: AsrResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.text, "hello");
        assert_eq!(back.start_ms, 100);
    }

    #[test]
    fn video_frame_serializes() {
        let f = VideoFrame {
            jpeg_base64: "AAAA".into(),
            timestamp_ms: 1000,
            width: 1280,
            height: 720,
        };
        let json = serde_json::to_string(&f).unwrap();
        let back: VideoFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(back.width, 1280);
    }
}
