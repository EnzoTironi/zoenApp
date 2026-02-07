"use client";

import React, { useState, useEffect, useCallback } from "react";
import { useToast } from "@/components/ui/use-toast";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Plus,
  MoreVertical,
  Trash2,
  Copy,
  Play,
  Pause,
  Calendar,
  AppWindow,
  Clock,
  Zap,
  Bell,
  FileText,
  Target,
  Webhook,
  Tag,
  ChevronRight,
  Loader2,
} from "lucide-react";
import type {
  Playbook,
  Trigger,
  Action,
  CreatePlaybookRequest,
} from "@/lib/types/playbook";
import { PlaybookEditor } from "@/components/playbooks/playbook-editor";
import { PlaybookTemplates } from "@/components/playbooks/playbook-templates";
import { PlaybookExecutionLog } from "@/components/playbooks/playbook-execution-log";
import { cn } from "@/lib/utils";

const API_BASE = "http://localhost:3030";

export default function PlaybooksPage() {
  const { toast } = useToast();
  const [playbooks, setPlaybooks] = useState<Playbook[]>([]);
  const [templates, setTemplates] = useState<Playbook[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedPlaybook, setSelectedPlaybook] = useState<Playbook | null>(null);
  const [isEditorOpen, setIsEditorOpen] = useState(false);
  const [isTemplatesOpen, setIsTemplatesOpen] = useState(false);
  const [activeTab, setActiveTab] = useState("all");

  // Fetch playbooks
  const fetchPlaybooks = useCallback(async () => {
    try {
      const response = await fetch(`${API_BASE}/playbooks`);
      if (!response.ok) throw new Error("Failed to fetch playbooks");
      const data = await response.json();
      setPlaybooks(data.playbooks || []);
    } catch (error) {
      console.error("Error fetching playbooks:", error);
      toast({
        title: "Error",
        description: "Failed to load playbooks. Is the server running?",
        variant: "destructive",
      });
    }
  }, [toast]);

  // Fetch templates
  const fetchTemplates = useCallback(async () => {
    try {
      const response = await fetch(`${API_BASE}/playbooks/templates`);
      if (!response.ok) throw new Error("Failed to fetch templates");
      const data = await response.json();
      setTemplates(data.templates || []);
    } catch (error) {
      console.error("Error fetching templates:", error);
    }
  }, []);

  // Initial load
  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      await Promise.all([fetchPlaybooks(), fetchTemplates()]);
      setLoading(false);
    };
    loadData();
  }, [fetchPlaybooks, fetchTemplates]);

  // Toggle playbook enabled state
  const togglePlaybook = async (id: string, enabled: boolean) => {
    try {
      const response = await fetch(`${API_BASE}/playbooks/${id}/toggle`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
      });

      if (!response.ok) throw new Error("Failed to toggle playbook");

      const updated = await response.json();
      setPlaybooks((prev) =>
        prev.map((p) => (p.id === id ? updated : p))
      );

      toast({
        title: enabled ? "Playbook enabled" : "Playbook disabled",
        description: `"${updated.name}" is now ${enabled ? "active" : "inactive"}.`,
      });
    } catch (error) {
      console.error("Error toggling playbook:", error);
      toast({
        title: "Error",
        description: "Failed to update playbook status",
        variant: "destructive",
      });
    }
  };

  // Delete playbook
  const deletePlaybook = async (id: string) => {
    try {
      const response = await fetch(`${API_BASE}/playbooks/${id}`, {
        method: "DELETE",
      });

      if (!response.ok) throw new Error("Failed to delete playbook");

      setPlaybooks((prev) => prev.filter((p) => p.id !== id));
      toast({
        title: "Playbook deleted",
        description: "The playbook has been removed.",
      });
    } catch (error) {
      console.error("Error deleting playbook:", error);
      toast({
        title: "Error",
        description: "Failed to delete playbook",
        variant: "destructive",
      });
    }
  };

  // Create new playbook
  const createPlaybook = async (playbook: CreatePlaybookRequest) => {
    try {
      const response = await fetch(`${API_BASE}/playbooks`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(playbook),
      });

      if (!response.ok) throw new Error("Failed to create playbook");

      const created = await response.json();
      setPlaybooks((prev) => [...prev, created]);
      setIsEditorOpen(false);

      toast({
        title: "Playbook created",
        description: `"${created.name}" has been created successfully.`,
      });
    } catch (error) {
      console.error("Error creating playbook:", error);
      toast({
        title: "Error",
        description: "Failed to create playbook",
        variant: "destructive",
      });
    }
  };

  // Update playbook
  const updatePlaybook = async (id: string, updates: Partial<Playbook>) => {
    try {
      const response = await fetch(`${API_BASE}/playbooks/${id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(updates),
      });

      if (!response.ok) throw new Error("Failed to update playbook");

      const updated = await response.json();
      setPlaybooks((prev) =>
        prev.map((p) => (p.id === id ? updated : p))
      );
      setIsEditorOpen(false);

      toast({
        title: "Playbook updated",
        description: `"${updated.name}" has been updated.`,
      });
    } catch (error) {
      console.error("Error updating playbook:", error);
      toast({
        title: "Error",
        description: "Failed to update playbook",
        variant: "destructive",
      });
    }
  };

  // Duplicate playbook
  const duplicatePlaybook = async (playbook: Playbook) => {
    const { id, created_at, updated_at, ...rest } = playbook;
    await createPlaybook({
      ...rest,
      name: `${playbook.name} (Copy)`,
    });
  };

  // Filter playbooks
  const filteredPlaybooks = playbooks.filter((p) => {
    if (activeTab === "enabled") return p.enabled;
    if (activeTab === "disabled") return !p.enabled;
    if (activeTab === "builtin") return p.is_builtin;
    return true;
  });

  // Get trigger icon
  const getTriggerIcon = (trigger: Trigger) => {
    switch (trigger.type) {
      case "app_open":
        return AppWindow;
      case "time":
        return Clock;
      case "keyword":
        return Zap;
      case "context":
        return Calendar;
      default:
        return Zap;
    }
  };

  // Get action icon
  const getActionIcon = (action: Action) => {
    switch (action.type) {
      case "notify":
        return Bell;
      case "summarize":
        return FileText;
      case "focus_mode":
        return Target;
      case "tag":
        return Tag;
      case "webhook":
        return Webhook;
      default:
        return Zap;
    }
  };

  // Format trigger description
  const formatTriggerDescription = (trigger: Trigger): string => {
    switch (trigger.type) {
      case "app_open":
        return `When ${trigger.app_name} opens`;
      case "time":
        return trigger.description || `At ${trigger.cron}`;
      case "keyword":
        return `When "${trigger.pattern}" is detected`;
      case "context":
        return "When context matches";
      default:
        return "Unknown trigger";
    }
  };

  // Format action description
  const formatActionDescription = (action: Action): string => {
    switch (action.type) {
      case "notify":
        return `Send notification: ${action.title}`;
      case "summarize":
        return `Summarize last ${action.timeframe} minutes`;
      case "focus_mode":
        return action.enabled ? "Enable focus mode" : "Disable focus mode";
      case "tag":
        return `Tag with ${action.tags.join(", ")}`;
      case "webhook":
        return `Call webhook ${action.url}`;
      default:
        return "Unknown action";
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="container mx-auto py-8 px-4 max-w-6xl">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-3xl font-bold">Playbooks</h1>
          <p className="text-muted-foreground mt-1">
            Automate your workflow with triggers and actions
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            onClick={() => setIsTemplatesOpen(true)}
          >
            <Copy className="h-4 w-4 mr-2" />
            Templates
          </Button>
          <Button onClick={() => {
            setSelectedPlaybook(null);
            setIsEditorOpen(true);
          }}>
            <Plus className="h-4 w-4 mr-2" />
            New Playbook
          </Button>
        </div>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="mb-6">
        <TabsList>
          <TabsTrigger value="all">All ({playbooks.length})</TabsTrigger>
          <TabsTrigger value="enabled">
            Active ({playbooks.filter((p) => p.enabled).length})
          </TabsTrigger>
          <TabsTrigger value="disabled">
            Inactive ({playbooks.filter((p) => !p.enabled).length})
          </TabsTrigger>
          <TabsTrigger value="builtin">Built-in</TabsTrigger>
        </TabsList>
      </Tabs>

      {filteredPlaybooks.length === 0 ? (
        <Card className="text-center py-16">
          <CardContent>
            <div className="w-16 h-16 bg-muted rounded-full flex items-center justify-center mx-auto mb-4">
              <Zap className="h-8 w-8 text-muted-foreground" />
            </div>
            <h3 className="text-lg font-medium mb-2">No playbooks yet</h3>
            <p className="text-muted-foreground mb-6 max-w-sm mx-auto">
              Create your first playbook to automate tasks based on triggers like
              time, app usage, or keywords.
            </p>
            <div className="flex gap-2 justify-center">
              <Button
                variant="outline"
                onClick={() => setIsTemplatesOpen(true)}
              >
                Browse Templates
              </Button>
              <Button
                onClick={() => {
                  setSelectedPlaybook(null);
                  setIsEditorOpen(true);
                }}
              >
                Create Playbook
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4">
          {filteredPlaybooks.map((playbook) => (
            <Card
              key={playbook.id}
              className={cn(
                "group transition-all hover:shadow-md",
                playbook.enabled && "border-primary/20"
              )}
            >
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="flex items-start gap-3">
                    <div
                      className="w-10 h-10 rounded-lg flex items-center justify-center text-xl"
                      style={{
                        backgroundColor: playbook.color || "#3B82F6",
                        opacity: 0.1,
                      }}
                    >
                      <span>{playbook.icon || "⚡"}</span>
                    </div>
                    <div>
                      <div className="flex items-center gap-2">
                        <CardTitle className="text-lg">{playbook.name}</CardTitle>
                        {playbook.is_builtin && (
                          <Badge variant="secondary" className="text-xs">
                            Built-in
                          </Badge>
                        )}
                      </div>
                      {playbook.description && (
                        <CardDescription className="text-sm mt-0.5">
                          {playbook.description}
                        </CardDescription>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Switch
                      checked={playbook.enabled}
                      onCheckedChange={(checked) =>
                        togglePlaybook(playbook.id, checked)
                      }
                    />
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" size="icon">
                          <MoreVertical className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem
                          onClick={() => {
                            setSelectedPlaybook(playbook);
                            setIsEditorOpen(true);
                          }}
                        >
                          <Copy className="h-4 w-4 mr-2" />
                          Edit
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={() => duplicatePlaybook(playbook)}
                        >
                          <Copy className="h-4 w-4 mr-2" />
                          Duplicate
                        </DropdownMenuItem>
                        {!playbook.is_builtin && (
                          <DropdownMenuItem
                            className="text-destructive"
                            onClick={() => deletePlaybook(playbook.id)}
                          >
                            <Trash2 className="h-4 w-4 mr-2" />
                            Delete
                          </DropdownMenuItem>
                        )}
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                </div>
              </CardHeader>

              <CardContent className="pt-0">
                <div className="space-y-3">
                  <div className="flex items-center gap-2 text-sm">
                    <span className="text-muted-foreground">When:</span>
                    <div className="flex flex-wrap gap-1">
                      {playbook.triggers.map((trigger, i) => {
                        const Icon = getTriggerIcon(trigger);
                        return (
                          <Badge
                            key={i}
                            variant="outline"
                            className="flex items-center gap-1"
                          >
                            <Icon className="h-3 w-3" />
                            {formatTriggerDescription(trigger)}
                          </Badge>
                        );
                      })}
                    </div>
                  </div>

                  <div className="flex items-center gap-2 text-sm">
                    <span className="text-muted-foreground">Then:</span>
                    <div className="flex flex-wrap gap-1">
                      {playbook.actions.map((action, i) => {
                        const Icon = getActionIcon(action);
                        return (
                          <Badge
                            key={i}
                            variant="secondary"
                            className="flex items-center gap-1"
                          >
                            <Icon className="h-3 w-3" />
                            {formatActionDescription(action)}
                          </Badge>
                        );
                      })}
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Editor Dialog */}
      <Dialog open={isEditorOpen} onOpenChange={setIsEditorOpen}>
        <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
          <PlaybookEditor
            playbook={selectedPlaybook}
            onSave={(data) => {
              if (selectedPlaybook) {
                updatePlaybook(selectedPlaybook.id, data);
              } else {
                createPlaybook(data as CreatePlaybookRequest);
              }
            }}
            onCancel={() => setIsEditorOpen(false)}
          />
        </DialogContent>
      </Dialog>

      {/* Templates Dialog */}
      <Dialog open={isTemplatesOpen} onOpenChange={setIsTemplatesOpen}>
        <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
          <PlaybookTemplates
            templates={templates}
            onSelect={(template) => {
              setSelectedPlaybook(null);
              setIsTemplatesOpen(false);
              // Create from template
              createPlaybook({
                name: template.name,
                description: template.description,
                triggers: template.triggers,
                actions: template.actions,
                cooldown_minutes: template.cooldown_minutes,
                max_executions_per_day: template.max_executions_per_day,
                icon: template.icon,
                color: template.color,
              });
            }}
            onCancel={() => setIsTemplatesOpen(false)}
          />
        </DialogContent>
      </Dialog>
    </div>
  );
}
