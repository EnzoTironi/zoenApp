"use client";

import { useQuery } from "@tanstack/react-query";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import { formatNumber } from "@/lib/utils";
import {
  Activity,
  Film,
  Mic,
  Users,
  HardDrive,
  ArrowUpRight,
  ArrowDownRight,
} from "lucide-react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import Link from "next/link";
import { Button } from "@/components/ui/button";

function MetricCard({
  title,
  value,
  description,
  icon: Icon,
  trend,
}: {
  title: string;
  value: string | number;
  description?: string;
  icon: React.ElementType;
  trend?: { value: number; positive: boolean };
}) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-mono uppercase tracking-wide">
          {title}
        </CardTitle>
        <Icon className="h-4 w-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-mono font-bold">{value}</div>
        {description && (
          <p className="text-xs text-muted-foreground mt-1">{description}</p>
        )}
        {trend && (
          <div className="flex items-center gap-1 mt-2">
            {trend.positive ? (
              <ArrowUpRight className="h-3 w-3 text-success" />
            ) : (
              <ArrowDownRight className="h-3 w-3 text-destructive" />
            )}
            <span
              className={`text-xs font-mono ${
                trend.positive ? "text-success" : "text-destructive"
              }`}
            >
              {trend.value}%
            </span>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function ChartCard({
  title,
  data,
  dataKey,
  color,
}: {
  title: string;
  data: { date: string; count: number }[];
  dataKey: string;
  color: string;
}) {
  return (
    <Card className="col-span-2">
      <CardHeader>
        <CardTitle className="text-sm font-mono uppercase tracking-wide lowercase">
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="h-[200px]">
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={data}>
              <defs>
                <linearGradient id={`gradient-${dataKey}`} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor={color} stopOpacity={0.3} />
                  <stop offset="95%" stopColor={color} stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
              <XAxis
                dataKey="date"
                tickFormatter={(value) =>
                  new Date(value).toLocaleDateString("en-US", {
                    month: "short",
                    day: "numeric",
                  })
                }
                stroke="hsl(var(--muted-foreground))"
                fontSize={10}
                tickLine={false}
              />
              <YAxis
                stroke="hsl(var(--muted-foreground))"
                fontSize={10}
                tickLine={false}
                tickFormatter={(value) => formatNumber(value)}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: "hsl(var(--background))",
                  border: "1px solid hsl(var(--border))",
                  fontFamily: "JetBrains Mono, monospace",
                  fontSize: "12px",
                }}
                labelFormatter={(value) =>
                  new Date(value).toLocaleDateString("en-US", {
                    month: "long",
                    day: "numeric",
                    year: "numeric",
                  })
                }
              />
              <Area
                type="monotone"
                dataKey="count"
                stroke={color}
                fillOpacity={1}
                fill={`url(#gradient-${dataKey})`}
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </CardContent>
    </Card>
  );
}

export default function DashboardPage() {
  const { data: metrics, isLoading } = useQuery({
    queryKey: ["metrics"],
    queryFn: api.getMetrics,
  });

  if (isLoading) {
    return (
      <div className="p-8 space-y-8">
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          <Skeleton className="h-[120px]" />
          <Skeleton className="h-[120px]" />
          <Skeleton className="h-[120px]" />
          <Skeleton className="h-[120px]" />
        </div>
        <div className="grid gap-4 md:grid-cols-2">
          <Skeleton className="h-[300px]" />
          <Skeleton className="h-[300px]" />
        </div>
      </div>
    );
  }

  const mockTrend = [
    { date: "2024-01-01", count: 1000 },
    { date: "2024-01-02", count: 1200 },
    { date: "2024-01-03", count: 1100 },
    { date: "2024-01-04", count: 1400 },
    { date: "2024-01-05", count: 1300 },
    { date: "2024-01-06", count: 1600 },
    { date: "2024-01-07", count: 1500 },
  ];

  return (
    <div className="p-8 space-y-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-mono font-bold lowercase">dashboard</h1>
          <p className="text-muted-foreground font-mono text-sm">
            overview of your screenpipe instance
          </p>
        </div>
        <div className="flex gap-2">
          <Link href="/users">
            <Button variant="outline">manage users</Button>
          </Link>
          <Link href="/audit">
            <Button variant="outline">view audit logs</Button>
          </Link>
        </div>
      </div>

      {/* Metrics Grid */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <MetricCard
          title="total frames"
          value={formatNumber(metrics?.totalFrames || 0)}
          description="screen captures stored"
          icon={Film}
          trend={{ value: 12, positive: true }}
        />
        <MetricCard
          title="audio chunks"
          value={formatNumber(metrics?.totalAudioChunks || 0)}
          description="audio recordings stored"
          icon={Mic}
          trend={{ value: 8, positive: true }}
        />
        <MetricCard
          title="active users"
          value={metrics?.activeUsers || 0}
          description="users with access"
          icon={Users}
        />
        <MetricCard
          title="storage used"
          value={`${formatNumber(metrics?.storageUsed || 0)} GB`}
          description="total disk usage"
          icon={HardDrive}
        />
      </div>

      {/* Charts */}
      <div className="grid gap-4 md:grid-cols-2">
        <ChartCard
          title="frames captured (last 7 days)"
          data={metrics?.framesTrend || mockTrend}
          dataKey="frames"
          color="hsl(var(--foreground))"
        />
        <ChartCard
          title="audio recorded (last 7 days)"
          data={metrics?.audioTrend || mockTrend}
          dataKey="audio"
          color="hsl(var(--muted-foreground))"
        />
      </div>

      {/* Quick Actions */}
      <Card>
        <CardHeader>
          <CardTitle className="text-sm font-mono uppercase tracking-wide lowercase">
            quick actions
          </CardTitle>
          <CardDescription>
            common administrative tasks
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-2">
            <Link href="/users">
              <Button variant="outline" className="gap-2">
                <Users className="h-4 w-4" />
                manage users
              </Button>
            </Link>
            <Link href="/audit">
              <Button variant="outline" className="gap-2">
                <Activity className="h-4 w-4" />
                view audit logs
              </Button>
            </Link>
            <Link href="/settings">
              <Button variant="outline" className="gap-2">
                <HardDrive className="h-4 w-4" />
                configure settings
              </Button>
            </Link>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
