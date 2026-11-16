# 打包与软件仓库

- 自研应用全部打成 .deb（每个应用在 apps/<name>/ 下维护 debian/ 打包文件，后续补齐）。
- 仓库用 aptly 建：三个发布通道 alpha / beta / stable，对应版本三阶段（Alpha / Beta / 正式版）。
- 仓库 GPG 签名，公钥包 dolphinos-archive-keyring 随系统预置。
- 日常小更新走 apt 正常推送；整版大升级靠 dolphinos-release 元包 + 升级脚本，设置里「检查更新」触发。
- Debian 官方 sid 源直接引用，自有源 pinning 优先级高于官方源，只覆盖自有包。
- 托管：自有机器经隧道对外提供 http(s) 仓库目录；ISO 不进仓库、也不放 R2，走 GitHub Releases + 官网 /os 直链。
