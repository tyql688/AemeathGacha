import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

function App() {
  const [logs, setLogs] = useState<string[]>([]);
  const [scanning, setScanning] = useState(false);
  const [urlFound, setUrlFound] = useState<string | null>(null);

  useEffect(() => {
    const unlisten = listen<string>("log-message", (event) => {
      setLogs((prev) => [...prev, event.payload]);
    });
    return () => { unlisten.then((f) => f()); };
  }, []);

  const startScan = async () => {
    setScanning(true);
    setLogs([]);
    setUrlFound(null);
    try {
      const url = await invoke<string | null>("scan_gacha_url");
      if (url) {
        setUrlFound(url);
      } else {
        setLogs((prev) => [...prev, "❌ 未找到有效的抽卡链接。请确认：\n1. 已打开过游戏内的【抽卡历史记录】\n2. 翻阅了几页记录以生成日志"]);
      }
    } catch (error) {
      setLogs((prev) => [...prev, `❌ 发生错误: ${error}`]);
    } finally {
      setScanning(false);
    }
  };

  const closeWindow = () => getCurrentWindow().close();

  return (
    <div className="w-full h-full relative overflow-hidden">
      {/* 背景层 - 清晰 */}
      <div
        className="absolute inset-0 bg-cover bg-center"
        style={{
          backgroundImage: "url('/background.jpg')",
        }}
      />

      {/* 内容层 */}
      <div className="relative z-10 w-full h-full flex flex-col">
        {/* 标题栏 - 玻璃拟态 */}
        <div
          className="h-10 flex items-center justify-between px-4 bg-white/15 backdrop-blur-md border-b border-white/30"
          data-tauri-drag-region
        >
          {/* 左上角图标和标题 */}
          <div className="flex items-center gap-2 no-drag">
            <img src="/icon.png" alt="icon" className="w-5 h-5 rounded" />
            <span className="text-sm text-white font-medium drop-shadow-md">爱弥斯抽卡助手</span>
          </div>

          {/* 关闭按钮 */}
          <button
            onClick={closeWindow}
            className="w-8 h-8 flex items-center justify-center text-white/80 hover:bg-white/20 hover:text-white rounded-lg transition-all duration-150 no-drag"
            title="关闭"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" strokeWidth={2} viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* 主内容 */}
        <div className="flex-1 flex items-center justify-center px-6 py-6">
          <div className="w-[480px] backdrop-blur rounded-2xl overflow-hidden flex flex-col border border-white/20">
            {/* 卡片标题 */}
            <div className="px-6 pt-5 pb-2 text-center">
              <h1 className="text-xl font-semibold text-gray-800">爱弥斯抽卡助手</h1>
              <p className="text-xs text-gray-500 mt-1">鸣潮抽卡记录导出工具</p>
            </div>

            {/* 按钮 */}
            <div className="px-6 py-3">
              <button
                onClick={startScan}
                disabled={scanning}
                className={`
                  w-full py-2.5 rounded-lg font-medium text-sm transition-all duration-200 shadow-md
                  ${scanning
                    ? "bg-gray-400/80 text-white cursor-not-allowed"
                    : "bg-blue-500/90 hover:bg-blue-600 text-white hover:shadow-lg hover:-translate-y-0.5"
                  }
                `}
              >
                {scanning ? "扫描中..." : "一键获取链接"}
              </button>
            </div>

            {/* 日志区域 - 透明能看到背景 */}
            <div className="px-6 pb-4">
              <div
                className="h-[240px] rounded-xl p-4 overflow-y-auto border border-white/50 bg-black/5 backdrop-blur-sm"
              >
                {logs.length === 0 ? (
                  <div className="h-full flex items-center justify-center text-gray-700 text-sm">
                    点击按钮开始扫描
                  </div>
                ) : (
                  <div className="space-y-2">
                    {logs.map((log, index) => (
                      <div
                        key={index}
                        className={`text-sm leading-relaxed whitespace-pre-line ${
                          log.includes("✅")
                            ? "text-green-800 font-medium"
                            : log.includes("❌")
                            ? "text-red-700 font-medium"
                            : log.includes("⚠️")
                            ? "text-amber-800"
                            : "text-gray-800"
                        }`}
                      >
                        {log}
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* 状态 */}
              <div className="text-xs text-gray-700 text-center mt-3 font-medium">
                {urlFound ? (
                  <span className="text-green-700">✓ 链接已复制</span>
                ) : scanning ? (
                  <span className="text-blue-700">扫描中...</span>
                ) : (
                  <span>准备就绪</span>
                )}
              </div>
            </div>

            {/* 底部留白 */}
            <div className="h-4" />
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
