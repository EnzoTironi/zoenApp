"use client";

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api, type AuditLog } from "@/lib/api";
import { formatDate } from "@/lib/utils";
import { ChevronLeft, ChevronRight } from "lucide-react";

const ACTION_OPTIONS = [
  { value: "all", label: "all actions" },
  { value: "user.create", label: "user created" },
  { value: "user.update", label: "user updated" },
  { value: "user.delete", label: "user deleted" },
  { value: "user.login", label: "user login" },
  { value: "settings.update", label: "settings updated" },
  { value: "data.export", label: "data exported" },
  { value: "data.delete", label: "data deleted" },
];

export function AuditLogTable() {
  const [page, setPage] = useState(1);
  const [filters, setFilters] = useState({
    startDate: "",
    endDate: "",
    action: "all",
  });

  const limit = 20;

  const { data, isLoading } = useQuery({
    queryKey: ["audit-logs", page, filters],
    queryFn: () =>
      api.getAuditLogs({
        page,
        limit,
        ...(filters.startDate && { startDate: filters.startDate }),
        ...(filters.endDate && { endDate: filters.endDate }),
        ...(filters.action !== "all" && { action: filters.action }),
      }),
  });

  const getActionBadgeVariant = (action: string) => {
    if (action.includes("delete")) return "destructive";
    if (action.includes("create")) return "default";
    if (action.includes("update")) return "secondary";
    return "outline";
  };

  const handleFilterChange = (key: string, value: string) => {
    setFilters((prev) => ({ ...prev, [key]: value }));
    setPage(1);
  };

  if (isLoading) {
    return (
      <div className="space-y-2">
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
      </div>
    );
  }

  const logs = data?.logs || [];
  const total = data?.total || 0;
  const totalPages = Math.ceil(total / limit);

  return (
    <div className="space-y-4">
      {/* Filters */}
      <div className="flex flex-wrap gap-4">
        <div className="flex flex-col gap-1">
          <label className="text-xs font-mono uppercase text-muted-foreground">
            from
          </label>
          <Input
            type="date"
            value={filters.startDate}
            onChange={(e) => handleFilterChange("startDate", e.target.value)}
            className="w-[160px]"
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs font-mono uppercase text-muted-foreground">
            to
          </label>
          <Input
            type="date"
            value={filters.endDate}
            onChange={(e) => handleFilterChange("endDate", e.target.value)}
            className="w-[160px]"
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs font-mono uppercase text-muted-foreground">
            action
          </label>
          <Select
            value={filters.action}
            onValueChange={(value) => handleFilterChange("action", value)}
          >
            <SelectTrigger className="w-[180px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ACTION_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Table */}
      <div className="border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>timestamp</TableHead>
              <TableHead>user</TableHead>
              <TableHead>action</TableHead>
              <TableHead>resource</TableHead>
              <TableHead>ip address</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {logs.length === 0 ? (
              <TableRow>
                <TableCell
                  colSpan={5}
                  className="text-center text-muted-foreground py-8"
                >
                  no audit logs found
                </TableCell>
              </TableRow>
            ) : (
              logs.map((log) => (
                <TableRow key={log.id}>
                  <TableCell className="text-muted-foreground whitespace-nowrap">
                    {formatDate(log.createdAt)}
                  </TableCell>
                  <TableCell>
                    <span className="font-mono text-sm">{log.userEmail}</span>
                  </TableCell>
                  <TableCell>
                    <Badge variant={getActionBadgeVariant(log.action)}>
                      {log.action}
                    </Badge>
                  </TableCell>
                  <TableCell className="font-mono text-sm">
                    {log.resource}
                  </TableCell>
                  <TableCell className="text-muted-foreground font-mono text-xs">
                    {log.ipAddress}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {/* Pagination */}
      <div className="flex items-center justify-between">
        <div className="text-sm text-muted-foreground font-mono">
          showing {(page - 1) * limit + 1} to {Math.min(page * limit, total)} of{" "}
          {total} entries
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            disabled={page === 1}
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
            disabled={page >= totalPages}
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}
