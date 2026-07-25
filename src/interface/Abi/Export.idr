-- SPDX-License-Identifier: AGPL-3.0-or-later
module Abi.Export

import Abi.Types
import Abi.Foreign

%default total

-- We output a simple JSON-like string mapping the Idris2 ABI constants.
-- This guarantees the .twasm bridge stays perfectly aligned with Idris2 layout proofs.

export
dumpConfig : String
dumpConfig = 
  "{\"type\": \"Tile\", \"tileSize\": " ++ show TileSize ++ ", \"rgba16fChannels\": " ++ show (channelCount RGBA16F) ++ ", \"magic\": 1347179520}\n" ++
  "{\"type\": \"LayerStack\", \"maxLayers\": 256, \"maxLayerNameLen\": 256, \"magic\": 1347179348}\n"

main : IO ()
main = putStr dumpConfig
