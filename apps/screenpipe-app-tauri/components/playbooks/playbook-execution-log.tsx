"use client";

import React, { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Loader2, CheckCircle, XCircle, Clock } from "lucide-react";
import type { PlaybookExecution } from "@/lib/types/playbook";

interface PlaybookExecutionLogProps {
  playbookId?: string;
}

const API_BASE = "http://localhost:3030";

export function PlaybookExecutionLog({ playbookId }: PlaybookExecutionLogProps) {
  const [executions, setExecutions] = useState<PlaybookExecution[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchExecutions = async () => {
      try {
        const url = new URL(`${API_BASE}/playbooks/executions`);
        if (playbookId) {
          url.searchParams.append("playbook_id", playbookId);
        }
        url.searchParams.append("limit", "20");

        const response = await fetch(url.toString());
        if (!response.ok) throw new Error("Failed to fetch executions");
        const data = await response.json();
        setExecutions(data.executions || []);
      } catch (error) {
        console.error("Error fetching executions:", error);
      } finally {
        setLoading(false);
      }
    };

    fetchExecutions();
    const interval = setInterval(fetchExecutions, 5000);
    return () => clearInterval(interval);
  }, [playbookId]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (executions.length === 0) {
    return (
      <Card>
        <CardContent className="py-8 text-center text-muted-foreground">
          No executions yet
        </CardContent>
      </Card>
    );
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "completed":
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case "failed":
        return <XCircle className="h-4 w-4 text-red-500" />;
      case "running":
        return <Clock className="h-4 w-4 text-yellow-500 animate-pulse" />;
      default:
        return <Clock className="h-4 w-4 text-muted-foreground" />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "completed":
        return "bg-green-500/10 text-green-500 border-green-500/20";
      case "failed":
        return "bg-red-500/10 text-red-500 border-red-500/20";
      case "running":
        return "bg-yellow-500/10 text-yellow-500 border-yellow-500/20";
      default:
        return "bg-muted text-muted-foreground";
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Recent Executions</CardTitle>
      </CardHeader>
      <CardContent>
        <ScrollArea className="h-[300px]">
          <div className="space-y-2">
            {executions.map((execution) => (
              <div
                key={execution.id}
                className="flex items-center justify-between p-3 border rounded-lg"
              >
                <div className="flex items-center gap-3">
                  {getStatusIcon(execution.status)}
                  <div>
                    <p className="text-sm font-medium">
                      {new Date(execution.started_at).toLocaleString()}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      Triggered by {execution.triggered_by.type}
                    </p>
                  </div>
                </div>
                <Badge
                  variant="outline"
                  className={getStatusColor(execution.status)}
                >
                  {execution.status}
                </Badge>
              </div>
            ))}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
