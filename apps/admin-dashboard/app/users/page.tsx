"use client";

import { UserTable } from "@/components/users/user-table";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Plus, ArrowLeft } from "lucide-react";
import Link from "next/link";

export default function UsersPage() {
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
            <h1 className="text-2xl font-mono font-bold lowercase">user management</h1>
            <p className="text-muted-foreground font-mono text-sm">
              manage users and their access permissions
            </p>
          </div>
        </div>
        <Button className="gap-2">
          <Plus className="h-4 w-4" />
          add user
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-mono uppercase tracking-wide lowercase">
            all users
          </CardTitle>
          <CardDescription>
            view and manage all users in your organization
          </CardDescription>
        </CardHeader>
        <CardContent>
          <UserTable />
        </CardContent>
      </Card>
    </div>
  );
}
