/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */

type detectBlocksRequest = {
  action: string,
  url: string,
}

let handleDetectBlocks = (url: string): Js.Promise.t<array<PdfTool.block>> =>
  Webapi.Fetch.fetch(url)
  ->Js.Promise.then_(response => response->Webapi.Fetch.Response.arrayBuffer)
  ->Js.Promise.then_(PdfTool.detectBlocks)
  ->Js.Promise.catch(err => {
    Js.log2("detect blocks failed", err)
    Js.Promise.resolve([||])
  })

Js.log("Blocky Writer background service worker loaded")
