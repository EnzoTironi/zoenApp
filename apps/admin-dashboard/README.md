# Screenpipe Admin Dashboard

Enterprise admin dashboard for managing Screenpipe instances with multi-tenancy support.

## Features

- **Dashboard Overview**: View metrics including total frames, audio chunks, active users, and storage usage
- **User Management**: Invite, manage, and control user access with role-based permissions
- **Audit Logs**: Track all actions performed in the organization with filtering and pagination
- **Playbooks**: Create and manage automation playbooks based on triggers
- **Settings**: Configure tenant settings including data retention and feature flags

## Tech Stack

- **Framework**: Next.js 15 with App Router
- **Language**: TypeScript
- **Styling**: Tailwind CSS with custom B&W design system
- **UI Components**: Radix UI + shadcn/ui
- **Charts**: Recharts
- **State Management**: TanStack Query (React Query)
- **Authentication**: JWT-based auth with middleware protection

## Getting Started

### Prerequisites

- Node.js 18+
- Screenpipe server running on port 3030

### Installation

```bash
# Install dependencies
npm install

# Run development server
npm run dev
```

The dashboard will be available at `http://localhost:3001`

### Build for Production

```bash
npm run build
npm start
```

## Project Structure

```
app/
├── (dashboard)/
│   ├── page.tsx              # Dashboard overview
│   ├── users/page.tsx        # User management
│   ├── audit/page.tsx        # Audit logs
│   ├── playbooks/page.tsx    # Playbook management
│   └── settings/page.tsx     # Tenant settings
├── login/page.tsx            # Login page
├── layout.tsx                # Root layout with providers
├── globals.css               # Global styles
components/
├── ui/                       # shadcn/ui components
├── auth/                     # Auth components
├── users/                    # User-related components
└── audit/                    # Audit log components
lib/
├── api.ts                    # API client
└── utils.ts                  # Utility functions
```

## API Proxy Configuration

The dashboard proxies API requests to the Screenpipe server. Configure the target URL in `next.config.js`:

```javascript
async rewrites() {
  return [
    {
      source: '/api/:path*',
      destination: 'http://localhost:3030/admin/:path*',
    },
  ];
}
```

## Authentication

The dashboard uses JWT-based authentication:
- Token is stored in localStorage and cookies
- Middleware protects routes and redirects to login if not authenticated
- API client automatically includes Bearer token in requests

## Design System

The dashboard follows a black & white geometric minimalism design:
- Pure black (#000000) and white (#FFFFFF) colors
- No border radius (sharp corners)
- Monospace typography (JetBrains Mono)
- 1px black borders

## Available Scripts

- `npm run dev` - Start development server on port 3001
- `npm run build` - Build for production
- `npm start` - Start production server
- `npm run lint` - Run ESLint
