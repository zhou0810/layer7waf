import { BrowserRouter, Routes, Route } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Layout } from "@/components/Layout";
import { Dashboard } from "@/pages/Dashboard";
import { Logs } from "@/pages/Logs";
import { Rules } from "@/pages/Rules";
import { Config } from "@/pages/Config";
import { Metrics } from "@/pages/Metrics";
import { BotDetection } from "@/pages/BotDetection";
import { AntiScraping } from "@/pages/AntiScraping";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      staleTime: 2000,
    },
  },
});

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/" element={<Dashboard />} />
            <Route path="/logs" element={<Logs />} />
            <Route path="/rules" element={<Rules />} />
            <Route path="/bots" element={<BotDetection />} />
            <Route path="/anti-scraping" element={<AntiScraping />} />
            <Route path="/config" element={<Config />} />
            <Route path="/metrics" element={<Metrics />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  );
}
