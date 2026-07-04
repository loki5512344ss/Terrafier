// Terrafier script example — creates a world with noise terrain
let world = new_world("scripted_world", 42);

// Generate noise terrain
generate_heightmap_noise(world, 12345, 16.0, 6, "overworld");

// Apply some operations
apply_smooth(world, 2, 5.0, "overworld");
apply_erode(world, 1, 4.0, "overworld");

// Generate layers (caves, rivers, biomes, etc.)
generate_layers(world, "overworld");

// Save the world
save_world(world, "scripted_world.tfw");

print(info(world));
print("Done! World saved to scripted_world.tfw");
