"use client";

import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, Event } from "@tauri-apps/api/event";

export type ActionItemStatus = "pending" | "in_progress" | "done" | "cancelled";
export type ActionItemPriority = "low" | "medium" | "high" | "critical";
export type ActionItemSource = "meeting" | "email" | "chat" | "document" | string;

export interface ActionItem {
  id: string;
  text: string;
  assignee?: string;
  deadline?: string;
  source: ActionItemSource;
  source_id?: string;
  confidence: number;
  status: ActionItemStatus;
  priority: ActionItemPriority;
  created_at: string;
  updated_at: string;
  completed_at?: string;
  metadata?: Record<string, unknown>;
}

export interface ActionItemsQuery {
  status?: ActionItemStatus;
  source?: string;
  assignee?: string;
  from_date?: string;
  to_date?: string;
  limit?: number;
  offset?: number;
}

export interface ActionItemsStats {
  total: number;
  pending: number;
  in_progress: number;
  done: number;
  cancelled: number;
}

export interface ExportConfig {
  format: "json" | "todoist" | "notion" | "webhook";
  webhook_url?: string;
  api_token?: string;
  enabled: boolean;
}

export function useActionItems() {
  const [items, setItems] = useState<ActionItem[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [stats, setStats] = useState<ActionItemsStats>({
    total: 0,
    pending: 0,
    in_progress: 0,
    done: 0,
    cancelled: 0,
  });

  const fetchActionItems = useCallback(async (query?: ActionItemsQuery) => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await invoke<ActionItem[]>("get_action_items", { query });
      setItems(response);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch action items");
      console.error("Error fetching action items:", err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const fetchStats = useCallback(async () => {
    try {
      const response = await invoke<ActionItemsStats>("get_action_items_stats");
      setStats(response);
    } catch (err) {
      console.error("Error fetching action items stats:", err);
    }
  }, []);

  const updateStatus = useCallback(async (id: string, status: ActionItemStatus) => {
    try {
      const response = await invoke<ActionItem>("update_action_item_status", {
        id,
        status,
      });
      setItems((prev) =>
        prev.map((item) => (item.id === id ? { ...item, ...response } : item))
      );
      await fetchStats();
      return response;
    } catch (err) {
      console.error("Error updating action item status:", err);
      throw err;
    }
  }, [fetchStats]);

  const deleteActionItem = useCallback(async (id: string) => {
    try {
      await invoke("delete_action_item", { id });
      setItems((prev) => prev.filter((item) => item.id !== id));
      await fetchStats();
    } catch (err) {
      console.error("Error deleting action item:", err);
      throw err;
    }
  }, [fetchStats]);

  const exportActionItems = useCallback(async (ids: string[], config: ExportConfig) => {
    try {
      await invoke("export_action_items", { ids, config });
    } catch (err) {
      console.error("Error exporting action items:", err);
      throw err;
    }
  }, []);

  const extractFromTranscript = useCallback(async (transcript: string, sourceId?: string) => {
    try {
      const response = await invoke<ActionItem[]>("extract_action_items_from_transcript", {
        transcript,
        sourceId,
      });
      await fetchActionItems();
      await fetchStats();
      return response;
    } catch (err) {
      console.error("Error extracting action items:", err);
      throw err;
    }
  }, [fetchActionItems, fetchStats]);

  useEffect(() => {
    fetchActionItems();
    fetchStats();
  }, [fetchActionItems, fetchStats]);

  // Listen for real-time action item notifications
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await listen<ActionItem[]>(
        "action-items-extracted",
        (event: Event<ActionItem[]>) => {
          setItems((prev) => [...event.payload, ...prev]);
          fetchStats();
        }
      );
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [fetchStats]);

  return {
    items,
    isLoading,
    error,
    stats,
    fetchActionItems,
    fetchStats,
    updateStatus,
    deleteActionItem,
    exportActionItems,
    extractFromTranscript,
  };
}

export function useActionItemFilters() {
  const [filters, setFilters] = useState<ActionItemsQuery>({
    status: undefined,
    source: undefined,
    assignee: undefined,
    limit: 50,
    offset: 0,
  });

  const setFilter = useCallback(<K extends keyof ActionItemsQuery>(
    key: K,
    value: ActionItemsQuery[K]
  ) => {
    setFilters((prev) => ({ ...prev, [key]: value, offset: 0 }));
  }, []);

  const clearFilters = useCallback(() => {
    setFilters({
      status: undefined,
      source: undefined,
      assignee: undefined,
      limit: 50,
      offset: 0,
    });
  }, []);

  const nextPage = useCallback(() => {
    setFilters((prev) => ({
      ...prev,
      offset: (prev.offset || 0) + (prev.limit || 50),
    }));
  }, []);

  const prevPage = useCallback(() => {
    setFilters((prev) => ({
      ...prev,
      offset: Math.max(0, (prev.offset || 0) - (prev.limit || 50)),
    }));
  }, []);

  return {
    filters,
    setFilter,
    clearFilters,
    nextPage,
    prevPage,
  };
}
