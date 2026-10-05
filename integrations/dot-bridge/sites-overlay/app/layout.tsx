import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Beaver · dot 桥接测试",
  description: "私有事件通知与结果回写验证台",
  other: {
    "codex-preview": "development",
  },
  icons: {
    icon: "/favicon.svg",
    shortcut: "/favicon.svg",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="zh-CN">
      <body className="antialiased">{children}</body>
    </html>
  );
}
