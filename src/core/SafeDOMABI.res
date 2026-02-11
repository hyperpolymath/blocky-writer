/* SPDX-License-Identifier: AGPL-3.0-or-later WITH Palimpsest */
/* Idris-aligned ABI surface, modeled after:
   /mnt/eclipse/repos/rescript-ecosystem/packages/web/dom-mounter/src/ABI/Types.idr */

type domResult =
  | Mounted
  | MountPointNotFound
  | InvalidSelector
  | InvalidHTML

type mountResult =
  | MountedAt(Dom.element)
  | NotFound(string)
  | Failed(string)

let selectorMinLength = 1
let selectorMaxLength = 255
let htmlMaxLength = 1048576
