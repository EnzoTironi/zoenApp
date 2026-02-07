"use client";

import { useState, useEffect } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Skeleton } from "@/components/ui/skeleton";
import { ArrowLeft, Save } from "lucide-react";
import Link from "next/link";
import { api } from "@/lib/api";
import { toast } from "sonner";

export default function SettingsPage() {
  const queryClient = useQueryClient();
  const [formData, setFormData] = useState({
    name: "",
    retentionDays: 30,
    maxUsers: 10,
    features: {
      audioRecording: true,
      screenRecording: true,
      aiInsights: false,
    },
  });

  const { data: settings, isLoading } = useQuery({
    queryKey: ["settings"],
    queryFn: api.getSettings,
  });

  useEffect(() => {
    if (settings) {
      setFormData({
        name: settings.name,
        retentionDays: settings.retentionDays,
        maxUsers: settings.maxUsers,
        features: settings.features,
      });
    }
  }, [settings]);

  const mutation = useMutation({
    mutationFn: api.updateSettings,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
      toast.success("settings saved");
    },
    onError: () => {
      toast.error("failed to save settings");
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    mutation.mutate(formData);
  };

  if (isLoading) {
    return (
      <div className="p-8 space-y-6">
        <Skeleton className="h-8 w-[200px]" />
        <Skeleton className="h-[400px]" />
      </div>
    );
  }

  return (
    <div className="p-8 space-y-6">
      <div className="flex items-center gap-4">
        <Link href="/">
          <Button variant="outline" size="icon">
            <ArrowLeft className="h-4 w-4" />
          </Button>
        </Link>
        <div>
          <h1 className="text-2xl font-mono font-bold lowercase">settings</h1>
          <p className="text-muted-foreground font-mono text-sm">
            configure your tenant settings
          </p>
        </div>
      </div>

      <form onSubmit={handleSubmit} className="space-y-6">
        {/* General Settings */}
        <Card>
          <CardHeader>
            <CardTitle className="text-sm font-mono uppercase tracking-wide lowercase">
              general
            </CardTitle>
            <CardDescription>
              basic tenant configuration
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">tenant name</Label>
              <Input
                id="name"
                value={formData.name}
                onChange={(e) =>
                  setFormData((prev) => ({ ...prev, name: e.target.value }))
                }
                placeholder="My Organization"
              />
            </div>
          </CardContent>
        </Card>

        {/* Data Retention */}
        <Card>
          <CardHeader>
            <CardTitle className="text-sm font-mono uppercase tracking-wide lowercase">
              data retention
            </CardTitle>
            <CardDescription>
              control how long data is stored
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="retention">retention period (days)</Label>
              <Input
                id="retention"
                type="number"
                min={1}
                max={365}
                value={formData.retentionDays}
                onChange={(e) =>
                  setFormData((prev) => ({
                    ...prev,
                    retentionDays: parseInt(e.target.value) || 30,
                  }))
                }
              />
              <p className="text-xs text-muted-foreground font-mono">
                data older than this will be automatically deleted
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="maxUsers">max users</Label>
              <Input
                id="maxUsers"
                type="number"
                min={1}
                max={1000}
                value={formData.maxUsers}
                onChange={(e) =>
                  setFormData((prev) => ({
                    ...prev,
                    maxUsers: parseInt(e.target.value) || 10,
                  }))
                }
              />
            </div>
          </CardContent>
        </Card>

        {/* Features */}
        <Card>
          <CardHeader>
            <CardTitle className="text-sm font-mono uppercase tracking-wide lowercase">
              features
            </CardTitle>
            <CardDescription>
              enable or disable features for your organization
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label>audio recording</Label>
                <p className="text-xs text-muted-foreground font-mono">
                  allow capturing audio from microphones
                </p>
              </div>
              <Switch
                checked={formData.features.audioRecording}
                onCheckedChange={(checked) =>
                  setFormData((prev) => ({
                    ...prev,
                    features: { ...prev.features, audioRecording: checked },
                  }))
                }
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label>screen recording</Label>
                <p className="text-xs text-muted-foreground font-mono">
                  allow capturing screen content
                </p>
              </div>
              <Switch
                checked={formData.features.screenRecording}
                onCheckedChange={(checked) =>
                  setFormData((prev) => ({
                    ...prev,
                    features: { ...prev.features, screenRecording: checked },
                  }))
                }
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="space-y-0.5">
                <Label>ai insights</Label>
                <p className="text-xs text-muted-foreground font-mono">
                  enable ai-powered analysis features
                </p>
              </div>
              <Switch
                checked={formData.features.aiInsights}
                onCheckedChange={(checked) =>
                  setFormData((prev) => ({
                    ...prev,
                    features: { ...prev.features, aiInsights: checked },
                  }))
                }
              />
            </div>
          </CardContent>
        </Card>

        <div className="flex justify-end">
          <Button type="submit" className="gap-2" disabled={mutation.isPending}>
            <Save className="h-4 w-4" />
            {mutation.isPending ? "saving..." : "save changes"}
          </Button>
        </div>
      </form>
    </div>
  );
}
