"use client";

import { useState } from "react";
import { useActionItems, useActionItemFilters, ActionItem, ActionItemStatus, ActionItemPriority } from "@/lib/hooks/use-action-items";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  CheckCircle2,
  Circle,
  Clock,
  XCircle,
  MoreVertical,
  Trash2,
  Download,
  ExternalLink,
  RefreshCw,
  Filter,
  Calendar,
  User,
  Tag,
  AlertCircle,
} from "lucide-react";
import { format } from "date-fns";
import { useToast } from "@/components/ui/use-toast";
import { cn } from "@/lib/utils";

const statusConfig: Record<
  ActionItemStatus,
  { label: string; icon: React.ReactNode; color: string }
> = {
  pending: {
    label: "Pending",
    icon: <Circle className="h-4 w-4" />,
    color: "bg-yellow-500/10 text-yellow-600 border-yellow-500/20",
  },
  in_progress: {
    label: "In Progress",
    icon: <Clock className="h-4 w-4" />,
    color: "bg-blue-500/10 text-blue-600 border-blue-500/20",
  },
  done: {
    label: "Done",
    icon: <CheckCircle2 className="h-4 w-4" />,
    color: "bg-green-500/10 text-green-600 border-green-500/20",
  },
  cancelled: {
    label: "Cancelled",
    icon: <XCircle className="h-4 w-4" />,
    color: "bg-gray-500/10 text-gray-600 border-gray-500/20",
  },
};

const priorityConfig: Record<ActionItemPriority, { label: string; color: string }> = {
  low: { label: "Low", color: "bg-gray-500/10 text-gray-600" },
  medium: { label: "Medium", color: "bg-blue-500/10 text-blue-600" },
  high: { label: "High", color: "bg-orange-500/10 text-orange-600" },
  critical: { label: "Critical", color: "bg-red-500/10 text-red-600" },
};

function ActionItemCard({
  item,
  onStatusChange,
  onDelete,
  selected,
  onSelect,
}: {
  item: ActionItem;
  onStatusChange: (id: string, status: ActionItemStatus) => void;
  onDelete: (id: string) => void;
  selected: boolean;
  onSelect: (id: string, checked: boolean) => void;
}) {
  const { toast } = useToast();
  const status = statusConfig[item.status];
  const priority = priorityConfig[item.priority];

  const handleExport = async (format: "todoist" | "notion") => {
    toast({
      title: "Export initiated",
      description: `Exporting to ${format}...`,
    });
  };

  return (
    <Card className={cn("transition-all", item.status === "done" && "opacity-60")}>
      <CardContent className="p-4">
        <div className="flex items-start gap-3">
          <Checkbox
            checked={selected}
            onCheckedChange={(checked) => onSelect(item.id, checked as boolean)}
            className="mt-1"
          />
          <div className="flex-1 min-w-0">
            <div className="flex items-start justify-between gap-2">
              <p
                className={cn(
                  "text-sm font-medium leading-relaxed",
                  item.status === "done" && "line-through text-muted-foreground"
                )}
              >
                {item.text}
              </p>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                    <MoreVertical className="h-4 w-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onClick={() => onStatusChange(item.id, "pending")}>
                    <Circle className="mr-2 h-4 w-4" />
                    Mark as Pending
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => onStatusChange(item.id, "in_progress")}>
                    <Clock className="mr-2 h-4 w-4" />
                    Mark as In Progress
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => onStatusChange(item.id, "done")}>
                    <CheckCircle2 className="mr-2 h-4 w-4" />
                    Mark as Done
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem onClick={() => handleExport("todoist")}>
                    <ExternalLink className="mr-2 h-4 w-4" />
                    Export to Todoist
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => handleExport("notion")}>
                    <ExternalLink className="mr-2 h-4 w-4" />
                    Export to Notion
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    onClick={() => onDelete(item.id)}
                    className="text-destructive"
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    Delete
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>

            <div className="flex flex-wrap items-center gap-2 mt-2">
              <Badge variant="outline" className={cn("text-xs", status.color)}>
                {status.icon}
                <span className="ml-1">{status.label}</span>
              </Badge>
              <Badge variant="secondary" className={cn("text-xs", priority.color)}>
                {priority.label}
              </Badge>
              {item.assignee && (
                <Badge variant="outline" className="text-xs">
                  <User className="h-3 w-3 mr-1" />
                  {item.assignee}
                </Badge>
              )}
              {item.deadline && (
                <Badge variant="outline" className="text-xs">
                  <Calendar className="h-3 w-3 mr-1" />
                  {format(new Date(item.deadline), "MMM d")}
                </Badge>
              )}
              <Badge variant="outline" className="text-xs capitalize">
                <Tag className="h-3 w-3 mr-1" />
                {item.source}
              </Badge>
              {item.confidence > 0 && (
                <Badge
                  variant="outline"
                  className={cn(
                    "text-xs",
                    item.confidence >= 0.8
                      ? "bg-green-500/10 text-green-600"
                      : item.confidence >= 0.5
                      ? "bg-yellow-500/10 text-yellow-600"
                      : "bg-red-500/10 text-red-600"
                  )}
                >
                  <AlertCircle className="h-3 w-3 mr-1" />
                  {Math.round(item.confidence * 100)}%
                </Badge>
              )}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function StatsCard({
  title,
  value,
  description,
  icon,
}: {
  title: string;
  value: number;
  description?: string;
  icon: React.ReactNode;
}) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
        {icon}
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{value}</div>
        {description && (
          <p className="text-xs text-muted-foreground">{description}</p>
        )}
      </CardContent>
    </Card>
  );
}

export default function ActionItemsPage() {
  const {
    items,
    isLoading,
    stats,
    fetchActionItems,
    updateStatus,
    deleteActionItem,
  } = useActionItems();
  const { filters, setFilter, clearFilters } = useActionItemFilters();
  const [selectedItems, setSelectedItems] = useState<Set<string>>(new Set());
  const { toast } = useToast();

  const handleSelect = (id: string, checked: boolean) => {
    const newSelected = new Set(selectedItems);
    if (checked) {
      newSelected.add(id);
    } else {
      newSelected.delete(id);
    }
    setSelectedItems(newSelected);
  };

  const handleSelectAll = (checked: boolean) => {
    if (checked) {
      setSelectedItems(new Set(items.map((item) => item.id)));
    } else {
      setSelectedItems(new Set());
    }
  };

  const handleBulkStatusChange = async (status: ActionItemStatus) => {
    try {
      await Promise.all(
        Array.from(selectedItems).map((id) => updateStatus(id, status))
      );
      setSelectedItems(new Set());
      toast({
        title: "Status updated",
        description: `Updated ${selectedItems.size} items to ${statusConfig[status].label}`,
      });
    } catch (err) {
      toast({
        title: "Error",
        description: "Failed to update status",
        variant: "destructive",
      });
    }
  };

  const handleBulkDelete = async () => {
    try {
      await Promise.all(Array.from(selectedItems).map((id) => deleteActionItem(id)));
      setSelectedItems(new Set());
      toast({
        title: "Items deleted",
        description: `Deleted ${selectedItems.size} items`,
      });
    } catch (err) {
      toast({
        title: "Error",
        description: "Failed to delete items",
        variant: "destructive",
      });
    }
  };

  const filteredItems = items.filter((item) => {
    if (filters.status && item.status !== filters.status) return false;
    if (filters.source && item.source !== filters.source) return false;
    if (filters.assignee && item.assignee !== filters.assignee) return false;
    return true;
  });

  return (
    <div className="container mx-auto py-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Action Items</h1>
          <p className="text-muted-foreground">
            Automatically extracted tasks from your meetings and conversations
          </p>
        </div>
        <Button onClick={() => fetchActionItems()} variant="outline" size="sm">
          <RefreshCw className="mr-2 h-4 w-4" />
          Refresh
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-4">
        <StatsCard
          title="Total"
          value={stats.total}
          icon={<Circle className="h-4 w-4 text-muted-foreground" />}
        />
        <StatsCard
          title="Pending"
          value={stats.pending}
          description="Needs attention"
          icon={<Clock className="h-4 w-4 text-yellow-500" />}
        />
        <StatsCard
          title="In Progress"
          value={stats.in_progress}
          icon={<RefreshCw className="h-4 w-4 text-blue-500" />}
        />
        <StatsCard
          title="Completed"
          value={stats.done}
          icon={<CheckCircle2 className="h-4 w-4 text-green-500" />}
        />
      </div>

      <Tabs defaultValue="all" className="space-y-4">
        <div className="flex items-center justify-between">
          <TabsList>
            <TabsTrigger value="all">All</TabsTrigger>
            <TabsTrigger value="pending">Pending</TabsTrigger>
            <TabsTrigger value="in_progress">In Progress</TabsTrigger>
            <TabsTrigger value="done">Done</TabsTrigger>
          </TabsList>

          <div className="flex items-center gap-2">
            <Dialog>
              <DialogTrigger asChild>
                <Button variant="outline" size="sm">
                  <Filter className="mr-2 h-4 w-4" />
                  Filters
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Filter Action Items</DialogTitle>
                  <DialogDescription>
                    Narrow down your action items by various criteria
                  </DialogDescription>
                </DialogHeader>
                <div className="space-y-4 py-4">
                  <div className="space-y-2">
                    <Label>Status</Label>
                    <Select
                      value={filters.status || "all"}
                      onValueChange={(value) =>
                        setFilter("status", value === "all" ? undefined : (value as ActionItemStatus))
                      }
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="All statuses" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">All</SelectItem>
                        <SelectItem value="pending">Pending</SelectItem>
                        <SelectItem value="in_progress">In Progress</SelectItem>
                        <SelectItem value="done">Done</SelectItem>
                        <SelectItem value="cancelled">Cancelled</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="space-y-2">
                    <Label>Source</Label>
                    <Select
                      value={filters.source || "all"}
                      onValueChange={(value) =>
                        setFilter("source", value === "all" ? undefined : value)
                      }
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="All sources" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">All</SelectItem>
                        <SelectItem value="meeting">Meeting</SelectItem>
                        <SelectItem value="email">Email</SelectItem>
                        <SelectItem value="chat">Chat</SelectItem>
                        <SelectItem value="document">Document</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <Button variant="outline" onClick={clearFilters} className="w-full">
                    Clear Filters
                  </Button>
                </div>
              </DialogContent>
            </Dialog>

            {selectedItems.size > 0 && (
              <>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => handleBulkStatusChange("done")}
                >
                  <CheckCircle2 className="mr-2 h-4 w-4" />
                  Mark Done ({selectedItems.size})
                </Button>
                <Button
                  variant="destructive"
                  size="sm"
                  onClick={handleBulkDelete}
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  Delete
                </Button>
              </>
            )}
          </div>
        </div>

        <TabsContent value="all" className="space-y-4">
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
            </div>
          ) : filteredItems.length === 0 ? (
            <Card>
              <CardContent className="flex flex-col items-center justify-center py-12">
                <CheckCircle2 className="h-12 w-12 text-muted-foreground mb-4" />
                <p className="text-lg font-medium">No action items found</p>
                <p className="text-sm text-muted-foreground">
                  Action items will appear here when extracted from your meetings
                </p>
              </CardContent>
            </Card>
          ) : (
            <>
              <div className="flex items-center gap-2 px-2">
                <Checkbox
                  checked={
                    selectedItems.size === items.length && items.length > 0
                  }
                  onCheckedChange={(checked) => handleSelectAll(checked as boolean)}
                />
                <span className="text-sm text-muted-foreground">
                  Select all {items.length} items
                </span>
              </div>
              <div className="space-y-2">
                {filteredItems.map((item) => (
                  <ActionItemCard
                    key={item.id}
                    item={item}
                    onStatusChange={updateStatus}
                    onDelete={deleteActionItem}
                    selected={selectedItems.has(item.id)}
                    onSelect={handleSelect}
                  />
                ))}
              </div>
            </>
          )}
        </TabsContent>

        <TabsContent value="pending" className="space-y-4">
          {filteredItems
            .filter((item) => item.status === "pending")
            .map((item) => (
              <ActionItemCard
                key={item.id}
                item={item}
                onStatusChange={updateStatus}
                onDelete={deleteActionItem}
                selected={selectedItems.has(item.id)}
                onSelect={handleSelect}
              />
            ))}
        </TabsContent>

        <TabsContent value="in_progress" className="space-y-4">
          {filteredItems
            .filter((item) => item.status === "in_progress")
            .map((item) => (
              <ActionItemCard
                key={item.id}
                item={item}
                onStatusChange={updateStatus}
                onDelete={deleteActionItem}
                selected={selectedItems.has(item.id)}
                onSelect={handleSelect}
              />
            ))}
        </TabsContent>

        <TabsContent value="done" className="space-y-4">
          {filteredItems
            .filter((item) => item.status === "done")
            .map((item) => (
              <ActionItemCard
                key={item.id}
                item={item}
                onStatusChange={updateStatus}
                onDelete={deleteActionItem}
                selected={selectedItems.has(item.id)}
                onSelect={handleSelect}
              />
            ))}
        </TabsContent>
      </Tabs>
    </div>
  );
}
