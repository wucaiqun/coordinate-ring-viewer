# Geo Ring Viewer — 分发说明

## 给同事的文件

打包后得到一个压缩包，例如：

`geo-ring-viewer-0.1.0-linux-x86_64.tar.gz`

里面包含：

- `geo-ring-viewer` — 可执行程序
- `examples/` — 示例坐标文件
- `README.md` — 使用说明

## 同事如何使用（Linux）

```bash
tar -xzf geo-ring-viewer-0.1.0-linux-x86_64.tar.gz
cd geo-ring-viewer-0.1.0-linux-x86_64
./geo-ring-viewer
```

建议在解压后的目录里运行（这样程序能找到同目录下的 `examples/`）。

也可双击运行（若桌面环境允许未打包的可执行文件）。

## 系统依赖（Linux）

本程序基于图形界面，一般需要已安装：

- X11 或 Wayland 桌面环境
- 常见库：`libxcb`、`libxkbcommon`、`libegl`、`libfontconfig` 等

在 Debian/Ubuntu 上若无法启动，可尝试：

```bash
sudo apt install libxcb-render0 libxcb-shape0 libxcb-xfixes0 \
  libxkbcommon0 libegl1 libfontconfig1
```

## 你自己如何打包

在项目根目录执行：

```bash
chmod +x package.sh
./package.sh
```

产物在 `dist/` 目录下。

## 跨平台说明

- 当前 `package.sh` 在 **你正在使用的系统** 上编译，得到的是 **本机架构** 的二进制（例如 Linux x86_64）。
- 若同事是 **Windows**，需在有 Windows 的环境执行 `cargo build --release`，或交叉编译后单独打包 `.exe`。
- 若同事是 **macOS**，需在 Mac 上编译后打包。

## 坐标格式（简要）

- 2D：每行 `经度 纬度`
- 3D：每行 `经度 纬度 高度(米)`
- 多个 ring：在同一文件里用一行 `===============` 分隔
- 导入：界面中 **Import 2D/3D from file(s)...**
