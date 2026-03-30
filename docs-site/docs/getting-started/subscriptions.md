---
title: Subscription Tiers
description: ThetaData subscription plans and what endpoints are available at each tier.
---

# Subscription Tiers

ThetaDataDx connects directly to [ThetaData's](https://thetadata.us/) market data infrastructure. Access to endpoints depends on your ThetaData subscription tier. This page helps you understand what is available at each level.

::: tip Official Pricing
For current pricing and to subscribe, visit [thetadata.us](https://thetadata.us/). ThetaDataDx is a client SDK, not a data provider. Your data access is determined by your ThetaData subscription.
:::

## Options Plans

| Feature | Value ($40/mo) | Standard ($80/mo) | Pro ($160/mo) |
|---------|:-:|:-:|:-:|
| Market coverage | 100% | 100% | 100% |
| US index and stock options | Yes | Yes | Yes |
| Real-time access | Yes | Yes | Yes |
| Unlimited requests | Yes | Yes | Yes |
| Request types | 3 | 7 | 12 |
| Historical data depth | 4 years | 8 years | 12 years |
| Low latency snapshots | Yes | Yes | Yes |
| Tick level data | -- | Yes | Yes |
| Option chain snapshots | -- | Yes (15K contracts) | Yes (all contracts) |
| Trade streams (FPSS) | -- | 15K contracts | Every option trade |
| Every NBBO quote (OPRA) | -- | Yes | Yes |

## Stocks Plans

| Feature | Value ($30/mo) | Standard ($80/mo) | Pro ($160/mo) |
|---------|:-:|:-:|:-:|
| Market coverage | 100% | 100% | 100% |
| US listed and OTC stocks | Yes | Yes | Yes |
| Real-time access | 15-min delay | Yes | Yes |
| Unlimited requests | Yes | Yes | Yes |
| Request types | 3 | 6 | 6 |
| Historical data depth | 4 years | 4y CTA / 8y UTP | 8y CTA / 12y UTP |
| Low latency snapshots | Yes | Yes | Yes |
| Data intervals | 15 min | 1 min | Tick level |
| Trade streams (FPSS) | -- | 1K contracts | Every stock trade |
| Every NBBO quote (OPRA) | -- | Yes | Yes |

## Endpoint Tier Requirements

Throughout this documentation, each endpoint page displays a **Tier** label indicating the minimum subscription required:

| Label | Meaning |
|-------|---------|
| **Free** | Available to all accounts, no paid subscription required |
| **Value+** | Requires Value tier or higher |
| **Standard+** | Requires Standard tier or higher |
| **Professional** | Requires Professional (Pro) tier |

If you call an endpoint that requires a higher tier than your subscription, the server will return an error. Consider upgrading your plan if you need access to additional endpoints.

## Endpoint Availability by Tier

### Free Tier

Available without a paid subscription:

- All **List** endpoints (symbols, dates, expirations, strikes, contracts, roots)
- **EOD** (end-of-day) history for stocks, options, and indices
- **Calendar** endpoints (open today, on date, year)
- **Interest rate** EOD history

### Value Tier

Everything in Free, plus:

- **Snapshot** endpoints for quotes and OHLC (stocks, options, indices)
- **History** endpoints for quotes and OHLC at configurable intervals
- **Open interest** history and snapshots

### Standard Tier

Everything in Value, plus:

- **Trade** snapshots and history (stocks and options)
- **Trade Quote** combined history
- **Index** snapshots and OHLC history
- **Greeks** snapshots and history (implied volatility, first order)
- **Streaming** (FPSS) with capped contract limits

### Professional Tier

Everything in Standard, plus:

- **Greeks** all orders, second order, third order (snapshot and history)
- **Trade Greeks** (per-trade greeks computation)
- **Streaming** (FPSS) for all contracts, unlimited

## Further Reading

For the most up-to-date information on plans, pricing, and data coverage:

- [ThetaData Pricing](https://thetadata.us/) -- official subscription plans
- [ThetaData Documentation](https://docs.thetadata.us/) -- official API documentation and guides
- [ThetaData OpenAPI Spec](https://docs.thetadata.us/openapiv3.yaml) -- machine-readable endpoint specifications
