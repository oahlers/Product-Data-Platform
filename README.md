# Product Data Platform - Rust MVP

Local zero-cloud-cost MVP using Tokio, Axum, a five-minute scheduler, JSON input, deterministic specialist modules, and persisted JSON results.

## Run

```bash
cargo run
```

Server: `http://localhost:3000`

## Endpoints

```text
GET  /health
GET  /products
POST /process/DEMO-001
POST /process-all
GET  /results/DEMO-001
```

## Quick test

```bash
curl http://localhost:3000/health
curl -X POST http://localhost:3000/process/DEMO-001
curl http://localhost:3000/results/DEMO-001
```

## Scheduler

`tokio-cron-scheduler` runs `process-all` every five minutes. The cron expression uses six fields with seconds first:

```text
0 */5 * * * *
```

## Current storage

- Input: `data/products.json`
- Results: `results/{SKU}.json`

This deliberately avoids Excel writes in the first MVP. Replace `JsonStore` with a SharePoint/Graph source and Azure SQL repository later without changing the orchestrator or API contract.

## Agent modules

The MVP contains deterministic local implementations for:

- Product Data Validator
- Text QA
- INCI Review
- Nordic Label Generator
- Translation placeholder

Put the final prompt text in `prompts/*.txt`. On Monday, add an `AiClient` implementation for Azure OpenAI and preserve the same `ModuleResult` JSON contracts.

## Azure migration mapping

```text
Local Axum container     -> Azure Container Apps or App Service
Local scheduler          -> Azure Functions Timer Trigger or Container Apps Job
products.json            -> Microsoft Graph/SharePoint + Azure SQL
results/*.json           -> Azure SQL + Blob Storage
Local module mocks       -> Azure OpenAI structured-output calls
Logs                     -> Application Insights
Secrets                  -> Key Vault + Managed Identity
```

## Important POC limitation

The local rules demonstrate orchestration and data flow, not legal or regulatory correctness. Specialist results remain neutral review signals.
