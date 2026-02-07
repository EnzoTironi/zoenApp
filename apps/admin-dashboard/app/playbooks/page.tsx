"use client";

import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { ArrowLeft, Plus, Play, Pause, Trash2, Edit } from "lucide-react";
import Link from "next/link";
import { toast } from "sonner";

interface Playbook {
  id: string;
  name: string;
  description: string;
  trigger: string;
  action: string;
  isActive: boolean;
  createdAt: string;
  lastRunAt?: string;
}

// Mock API - replace with actual API calls
const api = {
  getPlaybooks: async (): Promise<Playbook[]> => {
    // Replace with actual API call
    return [
      {
        id: "1",
        name: "Daily Summary",
        description: "Generate daily summary of screen activity",
        trigger: "schedule:daily",
        action: "generate_summary",
        isActive: true,
        createdAt: "2024-01-15T10:00:00Z",
        lastRunAt: "2024-01-16T08:00:00Z",
      },
      {
        id: "2",
        name: "Focus Time Alert",
        description: "Alert when focus time exceeds 2 hours",
        trigger: "condition:focus_time",
        action: "send_notification",
        isActive: false,
        createdAt: "2024-01-14T15:30:00Z",
      },
    ];
  },
  updatePlaybook: async (id: string, data: Partial<Playbook>): Promise<Playbook> => {
    // Replace with actual API call
    return { id, ...data } as Playbook;
  },
  deletePlaybook: async (id: string): Promise<void> => {
    // Replace with actual API call
    console.log("Deleting playbook", id);
  },
};

function PlaybookCard({
  playbook,
  onToggle,
  onDelete,
}: {
  playbook: Playbook;
  onToggle: (id: string, isActive: boolean) => void;
  onDelete: (id: string) => void;
}) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-start justify-between">
          <div>
            <CardTitle className="text-lg font-mono">{playbook.name}</CardTitle>
            <CardDescription>{playbook.description}</CardDescription>
          </div>
          <Badge variant={playbook.isActive ? "default" : "secondary"}>
            {playbook.isActive ? "active" : "inactive"}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span className="text-muted-foreground font-mono">trigger:</span>
            <p className="font-mono">{playbook.trigger}</p>
          </div>
          <div>
            <span className="text-muted-foreground font-mono">action:</span>
            <p className="font-mono">{playbook.action}</p>
          </div>
        </div>

        {playbook.lastRunAt && (
          <p className="text-xs text-muted-foreground font-mono">
            last run: {new Date(playbook.lastRunAt).toLocaleString()}
          </p>
        )}

        <div className="flex items-center justify-between pt-4 border-t">
          <div className="flex items-center gap-2">
            <Switch
              checked={playbook.isActive}
              onCheckedChange={(checked) => onToggle(playbook.id, checked)}
            />
            <span className="text-sm font-mono">
              {playbook.isActive ? "enabled" : "disabled"}
            </span>
          </div>
          <div className="flex gap-2">
            <Button variant="outline" size="icon">
              <Edit className="h-4 w-4" />
            </Button>
            <Button
              variant="outline"
              size="icon"
              onClick={() => onDelete(playbook.id)}
              className="text-destructive"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export default function PlaybooksPage() {
  const queryClient = useQueryClient();
  const [isCreateOpen, setIsCreateOpen] = useState(false);

  const { data: playbooks, isLoading } = useQuery({
    queryKey: ["playbooks"],
    queryFn: api.getPlaybooks,
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: Partial<Playbook> }) =>
      api.updatePlaybook(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["playbooks"] });
      toast.success("playbook updated");
    },
    onError: () => {
      toast.error("failed to update playbook");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: api.deletePlaybook,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["playbooks"] });
      toast.success("playbook deleted");
    },
    onError: () => {
      toast.error("failed to delete playbook");
    },
  });

  const handleToggle = (id: string, isActive: boolean) => {
    updateMutation.mutate({ id, data: { isActive } });
  };

  const handleDelete = (id: string) => {
    if (confirm("are you sure you want to delete this playbook?")) {
      deleteMutation.mutate(id);
    }
  };

  if (isLoading) {
    return (
      <div className="p-8 space-y-6">
        <Skeleton className="h-8 w-[200px]" />
        <div className="grid gap-4 md:grid-cols-2">
          <Skeleton className="h-[200px]" />
          <Skeleton className="h-[200px]" />
        </div>
      </div>
    );
  }

  return (
    <div className="p-8 space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link href="/">
            <Button variant="outline" size="icon">
              <ArrowLeft className="h-4 w-4" />
            </Button>
          </Link>
          <div>
            <h1 className="text-2xl font-mono font-bold lowercase">playbooks</h1>
            <p className="text-muted-foreground font-mono text-sm">
              automate actions based on triggers
            </p>
          </div>
        </div>
        <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
          <DialogTrigger asChild>
            <Button className="gap-2">
              <Plus className="h-4 w-4" />
              create playbook
            </Button>
          </DialogTrigger>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>create playbook</DialogTitle>
              <DialogDescription>
                create a new automation playbook
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <p className="text-sm text-muted-foreground">
                playbook creation form will be implemented here
              </p>
            </div>
          </DialogContent>
        </Dialog>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        {playbooks?.map((playbook) => (
          <PlaybookCard
            key={playbook.id}
            playbook={playbook}
            onToggle={handleToggle}
            onDelete={handleDelete}
          />
        ))}
      </div>

      {playbooks?.length === 0 && (
        <Card>
          <CardContent className="py-12 text-center">
            <p className="text-muted-foreground font-mono">
              no playbooks found. create your first playbook to get started.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
