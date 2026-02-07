"use client";

import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import { Skeleton } from "@/components/ui/skeleton";
import { api, type User } from "@/lib/api";
import { formatDate } from "@/lib/utils";
import { toast } from "sonner";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { MoreHorizontal, Trash2 } from "lucide-react";

export function UserTable() {
  const queryClient = useQueryClient();
  const [updatingId, setUpdatingId] = useState<string | null>(null);

  const { data: users, isLoading } = useQuery({
    queryKey: ["users"],
    queryFn: api.getUsers,
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: Partial<User> }) =>
      api.updateUser(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["users"] });
      toast.success("user updated");
    },
    onError: () => {
      toast.error("failed to update user");
    },
    onSettled: () => {
      setUpdatingId(null);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: api.deleteUser,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["users"] });
      toast.success("user deleted");
    },
    onError: () => {
      toast.error("failed to delete user");
    },
  });

  const handleToggleActive = (user: User) => {
    setUpdatingId(user.id);
    updateMutation.mutate({
      id: user.id,
      data: { isActive: !user.isActive },
    });
  };

  const handleDelete = (id: string) => {
    if (confirm("are you sure you want to delete this user?")) {
      deleteMutation.mutate(id);
    }
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

  if (!users?.length) {
    return (
      <div className="border border-border p-8 text-center text-muted-foreground font-mono text-sm">
        no users found
      </div>
    );
  }

  return (
    <div className="border border-border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>user</TableHead>
            <TableHead>role</TableHead>
            <TableHead>status</TableHead>
            <TableHead>created</TableHead>
            <TableHead>last login</TableHead>
            <TableHead className="w-[100px]">actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {users.map((user) => (
            <TableRow key={user.id}>
              <TableCell>
                <div className="flex flex-col">
                  <span className="font-mono text-sm">{user.name}</span>
                  <span className="font-mono text-xs text-muted-foreground">
                    {user.email}
                  </span>
                </div>
              </TableCell>
              <TableCell>
                <Badge variant={user.role === "admin" ? "default" : "secondary"}>
                  {user.role}
                </Badge>
              </TableCell>
              <TableCell>
                <div className="flex items-center gap-2">
                  <Switch
                    checked={user.isActive}
                    onCheckedChange={() => handleToggleActive(user)}
                    disabled={updatingId === user.id}
                  />
                  <span className="font-mono text-xs">
                    {user.isActive ? "active" : "inactive"}
                  </span>
                </div>
              </TableCell>
              <TableCell className="text-muted-foreground">
                {formatDate(user.createdAt)}
              </TableCell>
              <TableCell className="text-muted-foreground">
                {user.lastLoginAt ? formatDate(user.lastLoginAt) : "never"}
              </TableCell>
              <TableCell>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button variant="ghost" size="icon">
                      <MoreHorizontal className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem
                      onClick={() => handleDelete(user.id)}
                      className="text-destructive"
                    >
                      <Trash2 className="mr-2 h-4 w-4" />
                      delete
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
