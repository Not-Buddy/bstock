# Stock Predictor

A simple stock predictor in Rust that fetches historical data from Yahoo Finance, performs some analysis, and predicts future prices.

## Features

-   Fetches historical stock data from Yahoo Finance.
-   Calculates Simple Moving Average (SMA) and Exponential Moving Average (EMA).
-   Predicts future prices using simple linear regression.
-   Provides a command-line interface (CLI) to specify stock symbols and analysis period.
-   Displays the analysis results in a clean, table-based format with colors.
-   Shows a progress bar when fetching data for multiple stocks.

## How to Run

1.  Clone the repository.
2.  Install the Rust toolchain.
3.  Run the application using `cargo run`.

### Using the CLI

You can specify stock symbols and the analysis period using the command-line arguments:

```bash
cargo run -- -s <SYMBOLS> -p <PERIOD>
```

-   `-s, --symbols`: A list of stock symbols to analyze (e.g., `AAPL GOOGL MSFT`).
-   `-p, --period`: The analysis period in days (e.g., `90`).

If no arguments are provided, the application will use the symbols and period defined in the `stocks_config.json` file.

## Example Output
```
[00:00:04] ######################################## 5/5 (0s) Done
                         ─                  │
│ Stock Analysis for NVDA
│ Current Price             $179.39
─                         ├                  ┼
│ 10-day SMA                $187.70
─                         ├                  ┼
│ 50-day SMA                $186.46
─                         ├                  ┼
│ 20-day EMA                $187.37
─                         ├                  ┼
│ Prediction (Day 1)        $176.34
─                         ├                  ┼
│ Prediction (Day 2)        $174.95
─                         ├                  ┼
│ Prediction (Day 3)        $173.56
─                         ├                  ┼
│ Recent Trend              0.29%
                          ┤
```