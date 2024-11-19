# from https://github.com/tweag/nickel/issues/1864#issuecomment-2015333077
let fill_height_width = "height: 100%; width: 100%;" in
{
  example | String,
  features | Array String = [],
  index_html = m%"
      <!doctype html>
      <html lang="en" style="%{ fill_height_width } background-color: #2b2c2f;">

      <head>
          <meta charset="utf-8" />
          <title>haalka %{ example }</title>
          <link data-trunk rel="copy-dir" href="assets" />
          <link data-trunk rel="rust" data-cargo-features="debug,deployed_wasm_example,%{ features |> std.string.join "," }" data-wasm-opt="s" href="." />
      </head>

      <body style="%{ fill_height_width } margin: 0px">
          <div id="game" style="%{ fill_height_width }">
              <canvas id="bevy">
                  js and canvas support is required
              </canvas>
          </div>
      </body>

      </html>
  "%
}
