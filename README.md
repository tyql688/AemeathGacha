# 爱弥斯抽卡助手

鸣潮抽卡记录导出工具

## 功能

- 一键获取鸣潮游戏抽卡链接
- 自动复制到剪贴板

## 使用

1. 打开游戏内的【抽卡历史记录】
2. 翻阅几页记录以生成日志
3. 点击应用按钮获取链接

## 下载

在 [Releases](../../releases) 页面下载最新版本

## 开发

```bash
# 安装依赖
npm install

# 开发运行
npm run tauri dev

# 构建
npm run tauri build
```

构建完成后，安装包位于：
- `src-tauri/target/release/bundle/nsis/*.exe` - 安装程序

## 技术栈

- Tauri v2
- React + TypeScript
- Tailwind CSS
