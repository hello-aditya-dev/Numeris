import type { Metadata } from "next";
import "./globals.css";
import { Toaster } from "@/components/ui/toaster";
import NumerisApp from "@/components/numeris/NumerisApp";

export const metadata: Metadata = {
  title: "Numeris — Professional Statistical Software",
  description:
    "Local-first statistical research environment. Built locally. Computed deterministically. Reproducible by design. No AI. No subscription. No cloud requirement.",
  keywords: ["statistics", "econometrics", "regression", "panel data", "research software"],
  authors: [{ name: "hello-aditya-dev" }],
  openGraph: {
    title: "Numeris",
    description: "Professional statistical software. Built locally. Computed deterministically. Reproducible by design.",
    siteName: "Numeris",
    type: "website",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className="antialiased">
        {children}
        <Toaster />
      </body>
    </html>
  );
}
