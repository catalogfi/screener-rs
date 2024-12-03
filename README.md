# Address Screening Service

A server application to check whether an address is blacklisted or not. The service uses a screener API to determine the blacklist status of provided addresses.

## Features

- Screen addresses to check blacklist status.
- Limit batch size for requests to avoid overload.

## Create Configuration File

- Create a config.json file in the root directory with the following structure:

```json
{
  "db_url": "postgres://postgres:root@localhost:5432/garden",
  "screener_api_key": "73fb6ee0-453c-apikey-e39478ebbf9a",
  "risk_score_limit": 10,
  "whitelisted_addresses": [
    "0x699A8B34420A2a3bA1817b2C061ed852448F4173",
    "0x699A8B34420A2a3bA1817b2C061ed852448F4171",
    "0x699A8B34420A2a3bA1817b2C061ed852448F4172"
  ],
  "request_batch_size": 3
}
```

Config Descriptions:

- db_url: PostgreSQL database connection string.
- screener_api_key: API key for the screener service.
- risk_score_limit: Threshold for blacklisting based on risk score.
- whitelisted_addresses: A list of addresses excluded from blacklist checks.
- request_batch_size: Maximum number of addresses allowed per screening request.

## API Endpoints

### Base URL: `/`

#### Health Check

- **Endpoint:** `GET /`
- **Description:** Returns a basic health check response to ensure the server is running.

#### Address Screening

- **Endpoint:** `POST /screening/addresses`
- **Description:** Accepts a list of addresses and returns their blacklist status.

##### Request Body Example:

```json
[
  { "address": "0x12345", "chain": "ethereum" },
  { "address": "0x67890", "chain": "bitcoin" }
]
```

##### Response Body Example:

```json
[
  { "address": "0x12345", "chain": "ethereum", "is_blacklisted": false },
  { "address": "0x67890", "chain": "bitcoin", "is_blacklisted": true }
]
```

#### Response Errors:

- **400 Bad Request:**

  ```json
  { "error": "Batch size limit exceeded" }
  ```

- **500 Internal Server Error:**

```json
{ "error": "Error: <error details>" }
```

## Setup Instructions

1. **Clone the Repository**
   Clone the project repository to your local machine.

   ```bash
   git clone <repository_url>
   cd <project_directory>
   ```

2. **Run the Project**
   Ensure Rust is installed on your system. If not, install it using Rustup.
   Then, install project dependencies.
   ```bash
   cargo build
   ```
3. **Run the Server**
   Start the server

   ```bash
   cargo run
   ```

4. **Access the Server on port 3000**

```

```
