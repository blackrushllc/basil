# GAME Object (obj-game) (work in progress)

The `GAME` object provides a minimal 2D micro game engine foundation for Basil.

## Usage

```basil
LET g@ = NEW GAME()
LET a@ = g@.Assets
LET i@ = g@.Input
LET d@ = g@.Draw

SUB init()
    a@.LoadTexture("player", "assets/player.png")
END SUB

SUB update(dt)
    IF i@.KeyDown("LEFT") THEN ...
END SUB

SUB draw()
    d@.Clear(0.1, 0.2, 0.3, 1.0)
    d@.Sprite("player", x, y)
END SUB

g@.Window(800, 600, "My Game")
g@.Run(init, update, draw)
```

## API Reference

### `GAME@` (Singleton-like)
- `g@.Window(width%, height%, title$)`: Creates a window and initializes the renderer.
- `g@.Run(initFn, updateFn, drawFn)`: Starts the game loop.
  - `initFn()`: Called once before the first frame.
  - `updateFn(dt#)`: Called every frame with the elapsed time in seconds.
  - `drawFn()`: Called every frame to issue drawing commands.
- `g@.Quit()`: Exits the game.
- `g@.Assets`: Returns the `ASSETS` proxy object.
- `g@.Input`: Returns the `INPUT` proxy object.
- `g@.Draw`: Returns the `DRAW` proxy object.

### `ASSETS@`
- `a@.LoadTexture(key$, path$)`: Loads a PNG image as a texture.

### `INPUT@`
- `i@.KeyDown(key$) -> ok%`: Returns 1 if the key is pressed, 0 otherwise.
  - Supported keys: `"UP"`, `"DOWN"`, `"LEFT"`, `"RIGHT"`, `"SPACE"`, `"A"`..`"Z"`.

### `DRAW@`
- `d@.Clear(r#, g#, b#, a#)`: Clears the screen with the specified color.
- `d@.Sprite(key$, x#, y#)`: Draws a sprite with the given texture key.

## Implementation Notes
- Uses `winit` for windowing and `wgpu` for hardware-accelerated rendering.
- Single-threaded render loop.
- Blocking `Run` call.
