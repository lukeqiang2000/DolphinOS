# DolphinOS

基于 Debian unstable（sid）的自研 Linux 发行版。规格见 docs/SPEC.md，第一阶段记录见 STAGE1-REPORT.md。

目录：

- image/：live-build 镜像构建配置（第一阶段 amd64），build.sh 一键构建。
- apps/：自研应用（Slint + Rust）。dolphin-shape 是 squircle 超椭圆共享库，shell 是桌面 Shell 骨架，其余为占位目录。
- packaging/：aptly 软件仓库与打包说明。
- docs/：规格与文档。
