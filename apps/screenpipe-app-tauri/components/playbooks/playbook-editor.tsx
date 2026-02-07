"use client";

import React, { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import {
  Plus,
  Trash2,
  Clock,
  AppWindow,
  Zap,
  Calendar,
  Bell,
  FileText,
  Target,
  Tag,
  Webhook,
  ChevronRight,
} from "lucide-react";
import type {
  Playbook,
  Trigger,
  Action,
  CreatePlaybookRequest,
} from "@/lib/types/playbook";
import { cn } from "@/lib/utils";

interface PlaybookEditorProps {
  playbook?: Playbook | null;
  onSave: (data: Partial<Playbook> | CreatePlaybookRequest) => void;
  onCancel: () => void;
}

const TRIGGER_TYPES = [
  { value: "app_open", label: "App Opens", icon: AppWindow },
  { value: "time", label: "Scheduled Time", icon: Clock },
  { value: "keyword", label: "Keyword Detected", icon: Zap },
  { value: "context", label: "Context Match", icon: Calendar },
];

const ACTION_TYPES = [
  { value: "notify", label: "Send Notification", icon: Bell },
  { value: "summarize", label: "Generate Summary", icon: FileText },
  { value: "focus_mode", label: "Toggle Focus Mode", icon: Target },
  { value: "tag", label: "Tag Content", icon: Tag },
  { value: "webhook", label: "Call Webhook", icon: Webhook },
];

const FOCUS_OPTIONS = [
  { value: "all", label: "Everything" },
  { value: "action_items", label: "Action Items" },
  { value: "decisions", label: "Decisions" },
  { value: "key_points", label: "Key Points" },
];

const OUTPUT_OPTIONS = [
  { value: "notification", label: "Notification" },
  { value: "clipboard", label: "Clipboard" },
  { value: "pipe", label: "Pipe" },
];

export function PlaybookEditor({ playbook, onSave, onCancel }: PlaybookEditorProps) {
  const [name, setName] = useState(playbook?.name || "");
  const [description, setDescription] = useState(playbook?.description || "");
  const [icon, setIcon] = useState(playbook?.icon || "⚡");
  const [color, setColor] = useState(playbook?.color || "#3B82F6");
  const [triggers, setTriggers] = useState<Trigger[]>(playbook?.triggers || []);
  const [actions, setActions] = useState<Action[]>(playbook?.actions || []);
  const [cooldownMinutes, setCooldownMinutes] = useState(
    playbook?.cooldown_minutes?.toString() || ""
  );

  const addTrigger = (type: Trigger["type"]) => {
    let newTrigger: Trigger;
    switch (type) {
      case "app_open":
        newTrigger = { type: "app_open", app_name: "" };
        break;
      case "time":
        newTrigger = { type: "time", cron: "0 9 * * 1-5" };
        break;
      case "keyword":
        newTrigger = { type: "keyword", pattern: "", source: "both" };
        break;
      case "context":
        newTrigger = { type: "context" };
        break;
      default:
        return;
    }
    setTriggers([...triggers, newTrigger]);
  };

  const updateTrigger = (index: number, updates: Partial<Trigger>) => {
    setTriggers(
      triggers.map((t, i) => (i === index ? { ...t, ...updates } as Trigger : t))
    );
  };

  const removeTrigger = (index: number) => {
    setTriggers(triggers.filter((_, i) => i !== index));
  };

  const addAction = (type: Action["type"]) => {
    let newAction: Action;
    switch (type) {
      case "notify":
        newAction = { type: "notify", title: "", message: "" };
        break;
      case "summarize":
        newAction = { type: "summarize", timeframe: 60 };
        break;
      case "focus_mode":
        newAction = { type: "focus_mode", enabled: true };
        break;
      case "tag":
        newAction = { type: "tag", tags: [], timeframe: 60 };
        break;
      case "webhook":
        newAction = { type: "webhook", url: "", method: "POST" };
        break;
      default:
        return;
    }
    setActions([...actions, newAction]);
  };

  const updateAction = (index: number, updates: Partial<Action>) => {
    setActions(
      actions.map((a, i) => (i === index ? { ...a, ...updates } as Action : a))
    );
  };

  const removeAction = (index: number) => {
    setActions(actions.filter((_, i) => i !== index));
  };

  const handleSave = () => {
    const data = {
      name,
      description,
      icon,
      color,
      triggers,
      actions,
      cooldown_minutes: cooldownMinutes ? parseInt(cooldownMinutes) : undefined,
    };
    onSave(data);
  };

  const isValid = name && triggers.length > 0 && actions.length > 0;

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold">
          {playbook ? "Edit Playbook" : "Create Playbook"}
        </h2>
        <p className="text-sm text-muted-foreground">
          Define when this playbook should run and what actions to take
        </p>
      </div>

      {/* Basic Info */}
      <div className="space-y-4">
        <div className="flex items-center gap-4">
          <div className="space-y-2">
            <Label>Icon</Label>
            <Input
              value={icon}
              onChange={(e) => setIcon(e.target.value)}
              className="w-20 text-center text-xl"
              maxLength={2}
            />
          </div>
          <div className="space-y-2 flex-1">
            <Label>Name</Label>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g., Daily Standup"
            />
          </div>
          <div className="space-y-2">
            <Label>Color</Label>
            <Input
              type="color"
              value={color}
              onChange={(e) => setColor(e.target.value)}
              className="w-20 h-10 p-1"
            />
          </div>
        </div>

        <div className="space-y-2">
          <Label>Description</Label>
          <Textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="What does this playbook do?"
            rows={2}
          />
        </div>
      </div>

      {/* Triggers */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <Zap className="h-4 w-4" />
            When (Triggers)
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {triggers.length === 0 && (
            <p className="text-sm text-muted-foreground">
              Add at least one trigger to activate this playbook
            </p>
          )}

          {triggers.map((trigger, index) => (
            <div
              key={index}
              className="border rounded-lg p-4 space-y-3"
            >
              <div className="flex items-center justify-between">
                <Badge variant="outline">
                  {TRIGGER_TYPES.find((t) => t.value === trigger.type)?.label}
                </Badge>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => removeTrigger(index)}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>

              {trigger.type === "app_open" && (
                <div className="space-y-2">
                  <Label>App Name</Label>
                  <Input
                    value={trigger.app_name}
                    onChange={(e) =>
                      updateTrigger(index, { app_name: e.target.value })
                    }
                    placeholder="e.g., zoom, chrome"
                  />
                  <Label>Window Title (optional)</Label>
                  <Input
                    value={trigger.window_name || ""}
                    onChange={(e) =>
                      updateTrigger(index, { window_name: e.target.value })
                    }
                    placeholder="e.g., meet.google.com"
                  />
                </div>
              )}

              {trigger.type === "time" && (
                <div className="space-y-2">
                  <Label>Cron Expression</Label>
                  <Input
                    value={trigger.cron}
                    onChange={(e) =>
                      updateTrigger(index, { cron: e.target.value })
                    }
                    placeholder="0 9 * * 1-5"
                  />
                  <p className="text-xs text-muted-foreground">
                    Format: minute hour day month weekday
                  </p>
                </div>
              )}

              {trigger.type === "keyword" && (
                <div className="space-y-2">
                  <Label>Pattern</Label>
                  <Input
                    value={trigger.pattern}
                    onChange={(e) =>
                      updateTrigger(index, { pattern: e.target.value })
                    }
                    placeholder="e.g., action item, TODO"
                  />
                  <Label>Source</Label>
                  <Select
                    value={trigger.source}
                    onValueChange={(value) =>
                      updateTrigger(index, {
                        source: value as "ocr" | "audio" | "both",
                      })
                    }
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="ocr">Screen (OCR)</SelectItem>
                      <SelectItem value="audio">Audio</SelectItem>
                      <SelectItem value="both">Both</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              )}

              {trigger.type === "context" && (
                <div className="space-y-2">
                  <Label>Time Range</Label>
                  <Input
                    value={trigger.time_range || ""}
                    onChange={(e) =>
                      updateTrigger(index, { time_range: e.target.value })
                    }
                    placeholder="e.g., 09:00-17:00"
                  />
                </div>
              )}
            </div>
          ))}

          <Select onValueChange={(value) => addTrigger(value as Trigger["type"])}>
            <SelectTrigger className="w-full">
              <Plus className="h-4 w-4 mr-2" />
              <SelectValue placeholder="Add trigger" />
            </SelectTrigger>
            <SelectContent>
              {TRIGGER_TYPES.map((type) => (
                <SelectItem key={type.value} value={type.value}>
                  <div className="flex items-center gap-2">
                    <type.icon className="h-4 w-4" />
                    {type.label}
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </CardContent>
      </Card>

      {/* Actions */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <ChevronRight className="h-4 w-4" />
            Then (Actions)
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {actions.length === 0 && (
            <p className="text-sm text-muted-foreground">
              Add at least one action to perform
            </p>
          )}

          {actions.map((action, index) => (
            <div
              key={index}
              className="border rounded-lg p-4 space-y-3"
            >
              <div className="flex items-center justify-between">
                <Badge variant="secondary">
                  {ACTION_TYPES.find((a) => a.value === action.type)?.label}
                </Badge>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => removeAction(index)}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>

              {action.type === "notify" && (
                <div className="space-y-2">
                  <Label>Title</Label>
                  <Input
                    value={action.title}
                    onChange={(e) =>
                      updateAction(index, { title: e.target.value })
                    }
                    placeholder="Notification title"
                  />
                  <Label>Message</Label>
                  <Textarea
                    value={action.message}
                    onChange={(e) =>
                      updateAction(index, { message: e.target.value })
                    }
                    placeholder="Notification message"
                    rows={2}
                  />
                </div>
              )}

              {action.type === "summarize" && (
                <div className="space-y-2">
                  <Label>Timeframe (minutes)</Label>
                  <Input
                    type="number"
                    value={action.timeframe}
                    onChange={(e) =>
                      updateAction(index, { timeframe: parseInt(e.target.value) })
                    }
                  />
                  <Label>Focus</Label>
                  <Select
                    value={action.focus || "all"}
                    onValueChange={(value) =>
                      updateAction(index, {
                        focus: value as "all" | "action_items" | "decisions" | "key_points",
                      })
                    }
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {FOCUS_OPTIONS.map((opt) => (
                        <SelectItem key={opt.value} value={opt.value}>
                          {opt.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <Label>Output</Label>
                  <Select
                    value={action.output || "notification"}
                    onValueChange={(value) =>
                      updateAction(index, {
                        output: value as "notification" | "clipboard" | "pipe",
                      })
                    }
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {OUTPUT_OPTIONS.map((opt) => (
                        <SelectItem key={opt.value} value={opt.value}>
                          {opt.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              )}

              {action.type === "focus_mode" && (
                <div className="space-y-2">
                  <Label>Mode</Label>
                  <Select
                    value={action.enabled ? "enable" : "disable"}
                    onValueChange={(value) =>
                      updateAction(index, { enabled: value === "enable" })
                    }
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="enable">Enable</SelectItem>
                      <SelectItem value="disable">Disable</SelectItem>
                    </SelectContent>
                  </Select>
                  <Label>Duration (minutes, 0 = indefinite)</Label>
                  <Input
                    type="number"
                    value={action.duration || 0}
                    onChange={(e) =>
                      updateAction(index, { duration: parseInt(e.target.value) })
                    }
                  />
                </div>
              )}

              {action.type === "tag" && (
                <div className="space-y-2">
                  <Label>Tags (comma-separated)</Label>
                  <Input
                    value={action.tags.join(", ")}
                    onChange={(e) =>
                      updateAction(index, {
                        tags: e.target.value.split(",").map((t) => t.trim()),
                      })
                    }
                    placeholder="meeting, important, follow-up"
                  />
                  <Label>Timeframe (minutes)</Label>
                  <Input
                    type="number"
                    value={action.timeframe}
                    onChange={(e) =>
                      updateAction(index, { timeframe: parseInt(e.target.value) })
                    }
                  />
                </div>
              )}

              {action.type === "webhook" && (
                <div className="space-y-2">
                  <Label>URL</Label>
                  <Input
                    value={action.url}
                    onChange={(e) =>
                      updateAction(index, { url: e.target.value })
                    }
                    placeholder="https://api.example.com/webhook"
                  />
                  <Label>Method</Label>
                  <Select
                    value={action.method}
                    onValueChange={(value) =>
                      updateAction(index, {
                        method: value as "GET" | "POST" | "PUT" | "DELETE",
                      })
                    }
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="GET">GET</SelectItem>
                      <SelectItem value="POST">POST</SelectItem>
                      <SelectItem value="PUT">PUT</SelectItem>
                      <SelectItem value="DELETE">DELETE</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              )}
            </div>
          ))}

          <Select onValueChange={(value) => addAction(value as Action["type"])}>
            <SelectTrigger className="w-full">
              <Plus className="h-4 w-4 mr-2" />
              <SelectValue placeholder="Add action" />
            </SelectTrigger>
            <SelectContent>
              {ACTION_TYPES.map((type) => (
                <SelectItem key={type.value} value={type.value}>
                  <div className="flex items-center gap-2">
                    <type.icon className="h-4 w-4" />
                    {type.label}
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </CardContent>
      </Card>

      {/* Settings */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Settings</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            <Label>Cooldown (minutes)</Label>
            <Input
              type="number"
              value={cooldownMinutes}
              onChange={(e) => setCooldownMinutes(e.target.value)}
              placeholder="Minimum time between executions"
            />
            <p className="text-xs text-muted-foreground">
              Prevent the playbook from running too frequently
            </p>
          </div>
        </CardContent>
      </Card>

      {/* Footer */}
      <div className="flex justify-end gap-2 pt-4">
        <Button variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button onClick={handleSave} disabled={!isValid}>
          {playbook ? "Update" : "Create"} Playbook
        </Button>
      </div>
    </div>
  );
}
