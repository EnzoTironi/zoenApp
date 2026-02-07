# Screenpipe API Authentication

This document describes how to authenticate with the Screenpipe API using JWT tokens or API keys.

## Overview

Screenpipe supports two authentication methods:
1. **JWT Bearer Tokens** - For user sessions (expires after 24 hours)
2. **API Keys** - For service-to-service authentication (long-lived)

Authentication is enabled by setting `JWT_SECRET` or any API key environment variable.

## Configuration

### JWT Secret

Set a strong secret for signing JWT tokens:

```bash
# Required for JWT authentication
export JWT_SECRET="your-super-secret-jwt-key-min-32-chars"
```

**Important**: In production, always set a strong, unique `JWT_SECRET`. If not set, a default development secret is used (not secure for production).

### API Keys

Configure API keys for different access levels:

```bash
# Admin API key - full access to all endpoints
export API_KEY_ADMIN="sk-admin-your-secure-admin-key"
export API_KEY_ADMIN_TENANT="admin-tenant"  # Optional: assign to tenant

# User API key - standard access
export API_KEY_USER="sk-user-your-secure-user-key"
export API_KEY_USER_TENANT="user-tenant"  # Optional: assign to tenant

# Readonly API key - read-only access
export API_KEY_READONLY="sk-readonly-your-secure-readonly-key"
export API_KEY_READONLY_TENANT="readonly-tenant"  # Optional: assign to tenant

# Tenant-specific API keys (dynamic)
export API_KEY_TENANT_ACME="sk-acme-tenant-specific-key"
export API_KEY_TENANT_CORP="sk-corp-tenant-specific-key"
```

### User Credentials (for JWT login)

Configure users who can obtain JWT tokens via login:

```bash
# Admin user
export ADMIN_USERNAME="admin"
export ADMIN_PASSWORD_HASH="$2b$12$..."  # bcrypt hash (recommended)
# Or for development only:
export ADMIN_PASSWORD="admin"

# Readonly user
export READONLY_USERNAME="readonly"
export READONLY_PASSWORD_HASH="$2b$12$..."  # bcrypt hash (recommended)
# Or for development only:
export READONLY_PASSWORD="readonly"
```

**Generate bcrypt hash:**
```bash
# Using Python
python3 -c "import bcrypt; print(bcrypt.hashpw(b'your-password', bcrypt.gensalt(12)).decode())"

# Using Node.js
node -e "console.log(require('bcrypt').hashSync('your-password', 12))"
```

## Obtaining a JWT Token

### Login Endpoint

**POST** `/api/auth/login`

Request body:
```json
{
  "username": "admin",
  "password": "your-password"
}
```

### Example: Login with curl

```bash
curl -X POST http://localhost:3030/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "admin"
  }'
```

Response:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIs...",
  "token_type": "Bearer",
  "expires_in": 86400,
  "user": {
    "user_id": "admin",
    "role": "admin"
  }
}
```

## Using JWT Tokens

Include the token in the `Authorization` header with the `Bearer` prefix:

```bash
curl http://localhost:3030/api/health \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."
```

### Example: Access protected endpoint

```bash
# Store token from login
TOKEN=$(curl -s -X POST http://localhost:3030/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin"}' | jq -r '.access_token')

# Use token for subsequent requests
curl http://localhost:3030/api/search \
  -H "Authorization: Bearer $TOKEN" \
  -G -d "q=meeting" -d "limit=10"
```

## Using API Keys

API keys can be provided in two ways:

### Method 1: X-API-Key Header (Recommended)

```bash
curl http://localhost:3030/api/search \
  -H "X-API-Key: sk-admin-your-secure-admin-key" \
  -G -d "q=meeting" -d "limit=10"
```

### Method 2: Authorization Header with ApiKey prefix

```bash
curl http://localhost:3030/api/search \
  -H "Authorization: ApiKey sk-admin-your-secure-admin-key" \
  -G -d "q=meeting" -d "limit=10"
```

## Role-Based Access Control

### Roles

| Role | Permissions |
|------|-------------|
| `admin` | Full access to all endpoints |
| `user` | Standard access (read/write, no admin functions) |
| `readonly` | Read-only access |

### Role Hierarchy

- **Admin** can access all endpoints
- **User** can access user and readonly endpoints
- **Readonly** can only access readonly endpoints

### Checking Current User

Some endpoints return the current authenticated user information:

```bash
curl http://localhost:3030/api/me \
  -H "Authorization: Bearer $TOKEN"
```

## Multi-Tenant Support

When using tenant-specific API keys or JWT tokens with tenant claims, all data is automatically scoped to that tenant.

### Tenant-Specific API Keys

```bash
# Access as ACME tenant
curl http://localhost:3030/api/search \
  -H "X-API-Key: sk-acme-tenant-specific-key" \
  -G -d "q=project"

# Access as CORP tenant
curl http://localhost:3030/api/search \
  -H "X-API-Key: sk-corp-tenant-specific-key" \
  -G -d "q=project"
```

Each tenant only sees their own data, even when querying the same endpoints.

## Complete Examples

### Example 1: Full JWT Flow

```bash
#!/bin/bash

# Configuration
API_URL="http://localhost:3030"
USERNAME="admin"
PASSWORD="admin"

# Step 1: Login and get token
RESPONSE=$(curl -s -X POST "$API_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\": \"$USERNAME\", \"password\": \"$PASSWORD\"}")

TOKEN=$(echo "$RESPONSE" | jq -r '.access_token')
echo "Got token: $TOKEN"

# Step 2: Use token to search
curl "$API_URL/api/search" \
  -H "Authorization: Bearer $TOKEN" \
  -G -d "q=important" -d "limit=5"

# Step 3: Use token to get health status
curl "$API_URL/api/health" \
  -H "Authorization: Bearer $TOKEN"
```

### Example 2: API Key Usage

```bash
#!/bin/bash

# Configuration
API_URL="http://localhost:3030"
API_KEY="sk-admin-your-secure-admin-key"

# Method 1: X-API-Key header
curl "$API_URL/api/search?q=meeting" \
  -H "X-API-Key: $API_KEY"

# Method 2: Authorization header
curl "$API_URL/api/frames?limit=10" \
  -H "Authorization: ApiKey $API_KEY"

# Method 3: Readonly API key (if configured)
READONLY_KEY="sk-readonly-your-secure-readonly-key"
curl "$API_URL/api/search?q=report" \
  -H "X-API-Key: $READONLY_KEY"
```

### Example 3: Python Client

```python
import requests

API_URL = "http://localhost:3030"
API_KEY = "sk-admin-your-secure-admin-key"

# Using API Key
headers = {"X-API-Key": API_KEY}
response = requests.get(f"{API_URL}/api/search", headers=headers, params={"q": "meeting"})
print(response.json())

# Using JWT (first login, then use token)
login_response = requests.post(f"{API_URL}/api/auth/login", json={
    "username": "admin",
    "password": "admin"
})
token = login_response.json()["access_token"]

headers = {"Authorization": f"Bearer {token}"}
response = requests.get(f"{API_URL}/api/search", headers=headers, params={"q": "project"})
print(response.json())
```

### Example 4: JavaScript/Node.js Client

```javascript
const axios = require('axios');

const API_URL = 'http://localhost:3030';
const API_KEY = 'sk-admin-your-secure-admin-key';

// Using API Key
async function searchWithApiKey(query) {
  const response = await axios.get(`${API_URL}/api/search`, {
    headers: { 'X-API-Key': API_KEY },
    params: { q: query }
  });
  return response.data;
}

// Using JWT
async function searchWithJwt(username, password, query) {
  // Login
  const loginResponse = await axios.post(`${API_URL}/api/auth/login`, {
    username,
    password
  });
  const token = loginResponse.data.access_token;

  // Search
  const response = await axios.get(`${API_URL}/api/search`, {
    headers: { 'Authorization': `Bearer ${token}` },
    params: { q: query }
  });
  return response.data;
}

// Usage
searchWithApiKey('meeting').then(console.log);
searchWithJwt('admin', 'admin', 'project').then(console.log);
```

## Error Responses

### Missing Credentials (401)

```json
{
  "error": "Missing authentication credentials",
  "success": false
}
```

### Invalid Token (401)

```json
{
  "error": "Invalid token",
  "success": false
}
```

### Expired Token (401)

```json
{
  "error": "Token expired",
  "success": false
}
```

### Insufficient Permissions (403)

```json
{
  "error": "Insufficient permissions",
  "success": false
}
```

## Security Best Practices

1. **Always use HTTPS in production** - Never send credentials over HTTP
2. **Use strong secrets** - JWT_SECRET should be at least 32 random characters
3. **Prefer bcrypt hashes** - Use `ADMIN_PASSWORD_HASH` instead of plaintext passwords
4. **Rotate API keys regularly** - Change keys periodically and revoke compromised ones
5. **Use minimal required role** - Don't use admin keys when readonly suffices
6. **Store secrets securely** - Use environment variables or secret management, never commit to git
7. **Enable rate limiting** - The API has built-in rate limiting (10 req/s default, 1 req/s for login)

## Environment Variable Reference

| Variable | Description | Required |
|----------|-------------|----------|
| `JWT_SECRET` | Secret key for JWT signing | Recommended |
| `API_KEY_ADMIN` | Admin-level API key | Optional |
| `API_KEY_USER` | User-level API key | Optional |
| `API_KEY_READONLY` | Readonly API key | Optional |
| `API_KEY_TENANT_{ID}` | Tenant-specific API key | Optional |
| `ADMIN_USERNAME` | Admin username for login | Optional |
| `ADMIN_PASSWORD_HASH` | Admin password (bcrypt) | Optional |
| `ADMIN_PASSWORD` | Admin password (plaintext, dev only) | Optional |
| `READONLY_USERNAME` | Readonly username | Optional |
| `READONLY_PASSWORD_HASH` | Readonly password (bcrypt) | Optional |
| `READONLY_PASSWORD` | Readonly password (plaintext, dev only) | Optional |
