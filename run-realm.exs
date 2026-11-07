# Boot mull (the small-forest driver) with the beargarden realm, from an engine checkout:
#
#     cd ~/src/eg/repos/small-forest/repos/engine
#     PORT=4061 WEB_PORT=4062 elixir --no-halt -S mix run --no-start ~/src/eg/beargarden/run-realm.exs
#
# `--no-start` matters, as for Söder's: these put_env calls must land before the
# application boots. Nothing here changes the driver; the realm is configuration.
#   SPAWN=<room>        start somewhere other than the manifest's start room
#   CHART_CACHE=<dir>   the walkmask cache (default ~/.beargarden/worlds/<PORT>/chart-cache)
#   PLAYER_STORE=<dir>  saved players (default ~/.beargarden/worlds/<PORT>/players)

realm = Path.expand("world", __DIR__)
port = System.get_env("PORT", "4061")
home = Path.expand("~/.beargarden/worlds/#{port}")

Application.put_env(:engine, :world_path, realm)
Application.put_env(:engine, :chart_path, System.get_env("CHART_PATH", Path.join(realm, "charts")))

chart_cache = System.get_env("CHART_CACHE", Path.join(home, "chart-cache"))
File.mkdir_p!(chart_cache)
Application.put_env(:engine, :chart_cache_paths, [chart_cache])

manifest_start =
  with {:ok, text} <- File.read(Path.join(realm, "realm.json")),
       %{"start_room" => room} <- :json.decode(text),
       do: room,
       else: (_ -> "plattan")

spawn_room = System.get_env("SPAWN", manifest_start) |> String.to_atom()
Application.put_env(:engine, :start_room, spawn_room)
Application.put_env(:engine, :mounts, [])
store = System.get_env("PLAYER_STORE", Path.join(home, "players"))
File.mkdir_p!(store)
Application.put_env(:engine, :player_store, store)

if System.get_env("CLOCK_START") == nil, do: System.put_env("CLOCK_START", "14:00")
if System.get_env("WEATHER_START") == nil, do: System.put_env("WEATHER_START", "fair")

{:ok, _} = Application.ensure_all_started(:engine)

IO.puts("""

  beargarden up on telnet #{port}
  world: #{realm}
  spawn: #{spawn_room}
""")

Process.sleep(:infinity)
