# armillary
A model viewer for Stardust XR which works great for hand tracking, pointers, and controllers!
> [!IMPORTANT]  
> Requires the [Stardust XR Server](https://github.com/StardustXR/server) to be running.

If you installed the Stardust XR server via:  
```note
sudo dnf group install stardust-xr
```
Or if you installed via the [installation script](https://github.com/cyberneticmelon/usefulscripts/blob/main/stardustxr_setup.sh), Armillary comes pre-installed

## How To Use
Run the command `armilliary`, or `armilliary_dev` followed by the path to a .glb file

In flatscreen mode, you can rotate the model via the scroll wheel, and move it via right click

In XR mode you can use the bottom grab ring to move it around, and rotate the model by spinning the top ring

## Manual Installation
Clone the repository and after the server is running:
```sh
cargo run -- test_model.glb
```

Supports the following formats:  .gltf, .glb, .obj, .stl, ASCII .ply
