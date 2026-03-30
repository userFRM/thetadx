---
title: Subscriptions
description: Endpoint access by ThetaData subscription tier.
---

# Subscriptions

Each endpoint requires a minimum [ThetaData](https://thetadata.us/) subscription tier. The tables below map every endpoint to its required tier. For pricing details, visit [thetadata.us](https://thetadata.us/).

Throughout this documentation, endpoint pages display tier badges like this:

<TierBadge tier="free" /> All tiers have access

<TierBadge tier="standard" /> Standard and Pro only

## Stock Endpoints

| Endpoint | Free | Value | Standard | Pro |
|----------|:----:|:-----:|:--------:|:---:|
| **List** | | | | |
| Symbols | x | x | x | x |
| Dates | x | x | x | x |
| **Snapshot** | | | | |
| OHLC | | x | x | x |
| Trade | | | x | x |
| Quote | | x | x | x |
| Market Value | | | x | x |
| **History** | | | | |
| EOD | x | x | x | x |
| OHLC | | x | x | x |
| Trade | | | x | x |
| Quote | | x | x | x |
| Trade Quote | | | x | x |
| **At-Time** | | | | |
| Trade | | | x | x |
| Quote | | x | x | x |

## Option Endpoints

| Endpoint | Free | Value | Standard | Pro |
|----------|:----:|:-----:|:--------:|:---:|
| **List** | | | | |
| Roots | x | x | x | x |
| Dates | x | x | x | x |
| Strikes | x | x | x | x |
| Expirations | x | x | x | x |
| Contracts | x | x | x | x |
| **Snapshot** | | | | |
| OHLC | | x | x | x |
| Trade | | | x | x |
| Quote | | x | x | x |
| Open Interest | | x | x | x |
| Greeks IV | | | x | x |
| Greeks 1st Order | | | x | x |
| Greeks All | | | | x |
| Greeks 2nd Order | | | | x |
| Greeks 3rd Order | | | | x |
| **History** | | | | |
| EOD | x | x | x | x |
| OHLC | | x | x | x |
| Trade | | | x | x |
| Quote | | x | x | x |
| Trade Quote | | | x | x |
| Open Interest | | x | x | x |
| Greeks EOD | | | x | x |
| Greeks IV | | | x | x |
| Greeks 1st Order | | | x | x |
| Greeks All | | | | x |
| Greeks 2nd Order | | | | x |
| Greeks 3rd Order | | | | x |
| Trade Greeks IV | | | | x |
| Trade Greeks 1st Order | | | | x |
| Trade Greeks All | | | | x |
| Trade Greeks 2nd Order | | | | x |
| Trade Greeks 3rd Order | | | | x |
| **At-Time** | | | | |
| Trade | | | x | x |
| OHLC | | x | x | x |

## Index Endpoints

| Endpoint | Free | Value | Standard | Pro |
|----------|:----:|:-----:|:--------:|:---:|
| **List** | | | | |
| Symbols | x | x | x | x |
| Dates | x | x | x | x |
| **Snapshot** | | | | |
| OHLC | | | x | x |
| Price | | | x | x |
| Market Value | | | x | x |
| **History** | | | | |
| EOD | x | x | x | x |
| OHLC | | | x | x |
| Price | | | x | x |
| **At-Time** | | | | |
| Price | | | x | x |

## Calendar and Rate Endpoints

| Endpoint | Free | Value | Standard | Pro |
|----------|:----:|:-----:|:--------:|:---:|
| Open Today | x | x | x | x |
| On Date | x | x | x | x |
| Year | x | x | x | x |
| Interest Rate EOD | x | x | x | x |

## Streaming (FPSS)

| Feature | Free | Value | Standard | Pro |
|---------|:----:|:-----:|:--------:|:---:|
| Quote stream | | | x | x |
| Trade stream | | | x | x |
| Open Interest stream | | | x | x |
| OHLC stream | | | x | x |
| Contract limit | | | Capped | Unlimited |

::: tip
For the most up-to-date tier requirements, refer to ThetaData's official [Subscriptions page](https://docs.thetadata.us/Articles/Getting-Started/Subscriptions.html) and [OpenAPI spec](https://docs.thetadata.us/openapiv3.yaml).
:::
