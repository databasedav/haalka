# from https://github.com/tweag/nickel/issues/1864#issuecomment-2015333077
{
  example | String,
  index_html = m%"
      <!doctype html>
      <html lang="en">

      <head>
          <meta charset="utf-8" />
          <title>haalka %{ example }</title>
          <link data-trunk rel="copy-dir" href="assets" />
          <link data-trunk rel="rust" data-cargo-features="debug" data-wasm-opt="s" href="." />
      </head>

      <body>
          <div id="game" class="center" style="background-color: #2b2c2f;">
              <canvas id="bevy">
                  js and canvas support is required
              </canvas>
          </div>
      </body>

      </html>
  "%
}
