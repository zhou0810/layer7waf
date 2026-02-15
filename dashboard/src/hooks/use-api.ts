import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type AppConfig, type TestRuleRequest } from "@/lib/api";

export function useHealth() {
  return useQuery({
    queryKey: ["health"],
    queryFn: api.getHealth,
    refetchInterval: 5000,
  });
}

export function useStats() {
  return useQuery({
    queryKey: ["stats"],
    queryFn: api.getStats,
    refetchInterval: 3000,
  });
}

export function useMetrics() {
  return useQuery({
    queryKey: ["metrics"],
    queryFn: api.getMetrics,
    refetchInterval: 10000,
  });
}

export function useConfig() {
  return useQuery({
    queryKey: ["config"],
    queryFn: api.getConfig,
  });
}

export function useUpdateConfig() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (config: AppConfig) => api.updateConfig(config),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["config"] });
    },
  });
}

export function useRules() {
  return useQuery({
    queryKey: ["rules"],
    queryFn: api.getRules,
  });
}

export function useAddRule() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (rule: string) => api.addRule(rule),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["rules"] });
    },
  });
}

export function useDeleteRule() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: number) => api.deleteRule(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["rules"] });
    },
  });
}

export function useTestRule() {
  return useMutation({
    mutationFn: (data: TestRuleRequest) => api.testRule(data),
  });
}

export function useLogs(params?: {
  limit?: number;
  offset?: number;
  ip?: string;
  rule_id?: string;
  autoRefresh?: boolean;
}) {
  return useQuery({
    queryKey: ["logs", params?.limit, params?.offset, params?.ip, params?.rule_id],
    queryFn: () =>
      api.getLogs({
        limit: params?.limit,
        offset: params?.offset,
        ip: params?.ip,
        rule_id: params?.rule_id,
      }),
    refetchInterval: params?.autoRefresh ? 5000 : false,
  });
}
