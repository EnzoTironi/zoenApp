"use client";

import React from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { ArrowRight, Clock, AppWindow, Zap, Calendar } from "lucide-react";
import type { Playbook, Trigger } from "@/lib/types/playbook";

interface PlaybookTemplatesProps {
  templates: Playbook[];
  onSelect: (template: Playbook) => void;
  onCancel: () => void;
}

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

export function PlaybookTemplates({ templates, onSelect, onCancel }: PlaybookTemplatesProps) {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold">Playbook Templates</h2>
        <p className="text-sm text-muted-foreground">
          Choose a pre-built template to get started quickly
        </p>
      </div>

      <div className="grid gap-4">
        {templates.map((template) => (
          <Card
            key={template.id}
            className="cursor-pointer hover:border-primary transition-colors"
            onClick={() => onSelect(template)}
          >
            <CardHeader className="pb-3">
              <div className="flex items-start justify-between">
                <div className="flex items-start gap-3">
                  <div
                    className="w-10 h-10 rounded-lg flex items-center justify-center text-xl"
                    style={{
                      backgroundColor: template.color || "#3B82F6",
                      opacity: 0.1,
                    }}
                  >
                    <span>{template.icon || "⚡"}</span>
                  </div>
                  <div>
                    <CardTitle className="text-base">{template.name}</CardTitle>
                    {template.description && (
                      <CardDescription className="text-sm mt-0.5">
                        {template.description}
                      </CardDescription>
                    )}
                  </div>
                </div>
                <Button variant="ghost" size="sm">
                  Use Template
                  <ArrowRight className="h-4 w-4 ml-1" />
                </Button>
              </div>
            </CardHeader>

            <CardContent className="pt-0">
              <div className="space-y-2">
                <div className="flex items-center gap-2 text-sm">
                  <span className="text-muted-foreground">When:</span>
                  <div className="flex flex-wrap gap-1">
                    {template.triggers.map((trigger, i) => {
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
                    {template.actions.map((action, i) => (
                      <Badge key={i} variant="secondary">
                        {action.type.replace("_", " ")}
                      </Badge>
                    ))}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="flex justify-end gap-2 pt-4">
        <Button variant="outline" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
