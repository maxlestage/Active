// Numbers from bench/RESULTS.md (Chromium 141, Node 22, React 19.3).
export const BENCH = {
  size: [
    { name: "active-react", value: 0.74, us: true },
    { name: "react-head", value: 1.35 },
    { name: "@unhead/react", value: 6.07 },
    { name: "react-helmet-async", value: 6.29 },
    { name: "react-helmet", value: 6.76 },
  ],
  ssr: [
    { name: "React 19 (no library)", value: 39.7, base: true },
    { name: "active-react", value: 42.4, us: true },
    { name: "@unhead/react", value: 44.9 },
    { name: "react-head", value: 60.7 },
    { name: "react-helmet-async", value: 64.9 },
    { name: "react-helmet", value: 66.5 },
  ],
};
