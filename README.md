# StreamCache: Async Data Cache with Streaming Updates

**StreamCache** is an asynchronous caching system that periodically fetches data from an API and keeps it up-to-date via streaming updates. It’s designed to handle temperature data for various cities by maintaining a cache of the latest values, and it automatically updates based on real-time subscriptions.

## Features

- **Async API**: Uses an async trait `Api` to define the `fetch` and `subscribe` methods for obtaining data.
- **Background Updates**: Fetches data initially and then continuously updates via a background task.
- **Thread-Safe Cache**: Caches data using a thread-safe structure, `Arc<Mutex<HashMap<String, u64>>>`.
- **Error Handling**: Manages errors gracefully within data fetching and streaming.

## Usage

1. **Implement the `Api` Trait**: Your API should define `fetch` (initial data fetch) and `subscribe` (real-time updates) methods.
2. **Create a StreamCache Instance**: Instantiate `StreamCache` with your `Api` implementation. It will automatically start fetching and subscribing to updates.
3. **Retrieve Cached Data**: Use `get` to retrieve the latest temperature data for a city.

## Testing

Includes unit tests to verify:
- Successful data fetching and updating.
- Error handling in streaming subscriptions.
- Correct handling of missing or non-existent keys. 

