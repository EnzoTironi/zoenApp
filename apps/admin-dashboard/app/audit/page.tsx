"use client";

import { AuditLogTable } from "@/components/audit/audit-log-table";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ArrowLeft, Download } from "lucide-react";
import Link from "next/link";

export default function AuditPage() {
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
            <h1 className="text-2xl font-mono font-bold lowercase">audit logs</h1>
            <p className="text-muted-foreground font-mono text-sm">
              track all actions performed in your organization
            </p>
          </div>
        </div>
        <Button variant="outline" className="gap-2">
          <Download className="h-4 w-4" />
          export
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-mono uppercase tracking-wide lowercase">
            activity log
          </CardTitle>
          <CardDescription>
            filter and review all system activities
          </CardDescription>
        </CardHeader>
        <CardContent>
          <AuditLogTable />
        </CardContent>
      </Card>
    </div>
  );
}
