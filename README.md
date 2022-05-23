# url-shorter

A simple, low level url shorting service written with [`tiny_http`](https://github.com/tiny-http/tiny-http).\
The service is single threaded and uses at most 2 cpu cores under normal circumstances,
since multi-threading via a `RwLock` on `data` makes processing speed ~30% slower under the same cpu usage.\
**It's just a little side project, data integrity is not guaranteed!**

## API

### GET /\[id\]

O(1)\
Get a redirect by `[id]`.

<details>
<summary>302 Found</summary><br>

If the id is valid and the redirect exists,\
the server will respond as follows:

```http
HTTP/1.1 302 Found
Location: https://example.com/
Content-Length: 0

```

<br></details>

<details>
<summary>400 Invalid id</summary><br>

When passing an id with `[^0-9a-zA-Z]` in it, or the id decoded is larger than <code>2<sup>64</sup> − 1</code>,\
the server will respond as follows:

```http
HTTP/1.1 400 Bad Request
Content-Type: text/plain; charset=utf-8
Content-Length: 10

invalid-id
```

<br></details>

<details>
<summary>404 Not found</summary><br>

When passing an id that doesn't yet have a corresponding redirect or it has been deleted,\
the server will respond as follows:

```http
HTTP/1.1 404 Not Found
Content-Type: text/plain; charset=utf-8
Content-Length: 9

not-found
```

<br></details>

### GET /+\[url\]

O(1)\
Add a new redirect to `[url]`.

<details>
<summary>200 Success</summary><br>

If the provided url is valid,\
the server will respond as follows:

```http
HTTP/1.1 200 OK
Content-Type: application/json
Content-Length: 60

{"id":"yc5c","token":"ee7d345c-d526-4b4e-96fb-ec770197335d"}
```

Where `id` is the id used to access the redirect, and `token` is the token used to remove the redirect.

Note: Every successful request will cause a file write.

<br></details>

<details>
<summary>400 Invalid URL</summary><br>

If the URL can't be parsed by `Url::parse`,\
the server will respond as follows:

```http
HTTP/1.1 400 Bad Request
Content-Type: text/plain; charset=utf-8
Content-Length: 22

invalid-url:empty-host
```

Where the reason behind the colon is one of the following:

- `empty-host`
- `idna-error`
- `invalid-port`
- `invalid-ipv4`
- `invalid-ipv6`
- `invalid-domain`
- `relative-url-without-base`
- `relative-url-with-cannot-be-a-base-base`
- `set-host-on-cannot-be-a-base-url`
- `overflow`
- `unknown`

<br></details>

<details>
<summary>500 Write error</summary><br>

If the server fails to write the redirect to `data.txt`,\
the server will respond as follows:

```http
HTTP/1.1 500 Internal Server Error
Content-Type: text/plain; charset=utf-8
Content-Length: 11

write-error
```

<br></details>

### GET /-\[token\]

O(n)\
Remove a redirect by `[token]`.

<details>
<summary>200 Success</summary><br>

If the token is a valid UUID and the redirect exists,\
the server will respond as follows:

```http
HTTP/1.1 200 OK
Content-Type: text/plain; charset=utf-8
Content-Length: 20

https://example.com/
```

Where the response body is the url of the redirect.

Note: Every successful request will cause a file write to `prm.txt`, and a entire file rewrite to `data.txt` upon restart.

<br></details>

<details>
<summary>400 Invalid token</summary><br>

If the token is not a valid UUID,\
the server will respond as follows:

```http
HTTP/1.1 400 Bad Request
Content-Type: text/plain; charset=utf-8
Content-Length: 13

invalid-token
```

<br></details>

<details>
<summary>400 Not found</summary><br>

If the token doesn't have a corresponding redirect exists,\
the server will respond as follows:

```http
HTTP/1.1 400 Bad Request
Content-Type: text/plain; charset=utf-8
Content-Length: 9

not-found
```

<br></details>

<details>
<summary>500 Write error</summary><br>

If the server fails to write the token to `prm.txt`,\
the server will respond as follows:

```http
HTTP/1.1 500 Internal Server Error
Content-Type: text/plain; charset=utf-8
Content-Length: 11

write-error
```

<br></details>

## Benchmark

```bash
# Adding, heavily depends on I/O speed, so results may vary.
$ wrk -t1 -c4 -d30s --latency http://localhost:8000/+https://example.com/
Running 30s test @ http://localhost:8000/+https://example.com/
  1 threads and 4 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    88.89us  200.95us  14.92ms   99.90%
    Req/Sec    47.25k     4.07k   56.67k    69.44%
  Latency Distribution
     50%   82.00us
     75%   91.00us
     90%  101.00us
     99%  114.00us
  1414910 requests in 30.10s, 261.73MB read
Requests/sec:  47007.34
Transfer/sec:      8.70MB

# Querying, `t.lua` is provided on the root directory.
$ wrk -t1 -c4 -d120s -s t.lua --latency http://localhost:8000
Running 2m test @ http://localhost:8000
  1 threads and 4 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    28.34us   10.09us   3.93ms   94.84%
    Req/Sec   123.77k     8.03k  148.95k    69.42%
  Latency Distribution
     50%   28.00us
     75%   30.00us
     90%   33.00us
     99%   54.00us
  14780075 requests in 2.00m, 1.87GB read
Requests/sec: 123167.10
Transfer/sec:     15.97MB

# Removing, skipped empty url check, benchmarked against the last redirect (forced O(n), len 1414913).
$ wrk -t1 -c4 -d30s --latency http://localhost:8000/-5445b937-5157-4caa-a550-af7271bb81b2
Running 30s test @ http://localhost:8000/-5445b937-5157-4caa-a550-af7271bb81b2
  1 threads and 4 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    38.53ms    1.43ms  47.46ms   80.34%
    Req/Sec   104.22      5.15   111.00     59.67%
  Latency Distribution
     50%   38.87ms
     75%   39.64ms
     90%   39.85ms
     99%   41.41ms
  3113 requests in 30.01s, 431.71KB read
Requests/sec:    103.73
Transfer/sec:     14.38KB
```
