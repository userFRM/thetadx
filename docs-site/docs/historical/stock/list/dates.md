---
title: List Dates
description: Retrieve available dates for a stock by request type (Trade, Quote, OHLC).
---

# stock_list_dates

List available dates for a stock filtered by request type. Use this to discover what date range is available before requesting historical data.

<TierBadge tier="value" />

## Code Example

::: code-group
```rust [Rust]
let dates: Vec<String> = tdx.stock_list_dates("TRADE", "AAPL").await?;
println!("First: {} Last: {}", dates.first().unwrap(), dates.last().unwrap());
```
```python [Python]
dates = tdx.stock_list_dates("TRADE", "AAPL")
print(f"First: {dates[0]} Last: {dates[-1]}")
```
```go [Go]
dates, err := client.StockListDates("TRADE", "AAPL")
if err != nil {
    log.Fatal(err)
}
fmt.Printf("First: %s Last: %s\n", dates[0], dates[len(dates)-1])
```
```cpp [C++]
auto dates = client.stock_list_dates("TRADE", "AAPL");
std::cout << "First: " << dates.front()
          << " Last: " << dates.back() << std::endl;
```
:::

## Parameters

<div class="param-list">
<div class="param">
<div class="param-header"><code>request_type</code><span class="param-type">string</span><span class="param-badge required">required</span></div>
<div class="param-desc">Data type: <code>"TRADE"</code>, <code>"QUOTE"</code>, or <code>"OHLC"</code></div>
</div>
<div class="param">
<div class="param-header"><code>symbol</code><span class="param-type">string</span><span class="param-badge required">required</span></div>
<div class="param-desc">Ticker symbol (e.g. <code>"AAPL"</code>)</div>
</div>
</div>

## Response

List of date strings in `YYYYMMDD` format, sorted chronologically.

## Notes

- The available date range varies by request type.
- Use this endpoint to validate date ranges before making history requests.
