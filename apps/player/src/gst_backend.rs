//! GStreamer 播放后端骨架（feature "gst"，默认关闭）。
//!
//! 注意：本文件未在本机编译验证——开发机只有 libgstreamer 运行时库、
//! 没有 libgstreamer1.0-dev（pkg-config 找不到 gstreamer-1.0），
//! gstreamer crate 的构建脚本会失败。代码按 gstreamer 0.23 的公开
//! API 编写（playbin 元件），在装好 -dev 包的环境里启用
//! `cargo build -p dolphin-player --features gst` 后再做真机验证。

use crate::logic::{Backend, PlayState};
use gstreamer as gst;
use gstreamer::prelude::*;
use std::path::Path;

pub struct GstBackend {
    playbin: gst::Element,
    pub state: PlayState,
}

impl GstBackend {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        gst::init()?;
        let playbin = gst::ElementFactory::make("playbin").build()?;
        Ok(Self {
            playbin,
            state: PlayState::Stopped,
        })
    }
}

impl Backend for GstBackend {
    fn play(&mut self, path: &Path) {
        let uri = format!("file://{}", path.display());
        self.playbin.set_property("uri", &uri);
        let _ = self.playbin.set_state(gst::State::Playing);
        self.state = PlayState::Playing;
    }

    fn pause(&mut self) {
        let _ = self.playbin.set_state(gst::State::Paused);
        self.state = PlayState::Paused;
    }

    fn resume(&mut self) {
        let _ = self.playbin.set_state(gst::State::Playing);
        self.state = PlayState::Playing;
    }

    fn stop(&mut self) {
        let _ = self.playbin.set_state(gst::State::Null);
        self.state = PlayState::Stopped;
    }

    fn seek(&mut self, secs: f64) {
        let pos = gst::ClockTime::from_seconds_f64(secs);
        let _ = self
            .playbin
            .seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, pos);
    }
}
