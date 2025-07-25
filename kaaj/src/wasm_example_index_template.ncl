# from https://github.com/tweag/nickel/issues/1864#issuecomment-2015333077
let global_styles = "height: 100%; width: 100%; margin: 0; padding: 0; border: 0;" in
{
  library | String,
  example | String,
  features | Array String = [],
  has_assets | Bool = false,
  index_html = m%"
      <!doctype html>
      <html lang="en" style="%{ global_styles } background-color: #2b2c2f;">

      <head>
          <meta charset="utf-8" />
          <title>%{ library } %{ example } example</title>
          %{ if has_assets then m%"<link data-trunk rel="copy-dir" href="assets" />"% else "" }
          <link data-trunk rel="rust" data-cargo-features="debug,deployed_wasm_example,%{ features |> std.string.join "," }" data-wasm-opt="s" href="." />
      </head>

      <body style="%{ global_styles }">
          <div id="game" style="%{ global_styles } display: flex;">
              <canvas id="bevy">
                  js and canvas support is required
              </canvas>
          </div>
      </body>

      </html>
  "%
}
