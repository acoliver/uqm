import re

# Read the file
with open('src/graphics/scaling.rs', 'r') as f:
    content = f.read()

# Fix 1: Add the missing comment before ScaleCacheKey around line 1115
content = re.sub(
    r'(impl Default for BiadaptiveScaler \{[^}]+\}\n)(pub struct ScaleCacheKey)',
    r'\1\n// ==============================================================================\n// Scaling Cache\n// ==============================================================================\n\n\2',
    content,
    count=1
)

# Fix 2: Update ScalerManager struct to include biadaptive field
old_struct = r'''pub struct ScalerManager {
    /// Nearest-neighbor scaler
    nearest: NearestScaler,
    /// Bilinear scaler
    bilinear: BilinearScaler,
    /// Trilinear scaler
    trilinear: TrilinearScaler,
    /// HQ2x scaler
    hq2x: Hq2xScaler,
    /// Scaling cache
    cache: ScaleCache,
}'''

new_struct = r'''pub struct ScalerManager {
    /// Nearest-neighbor scaler
    nearest: NearestScaler,
    /// Bilinear scaler
    bilinear: BilinearScaler,
    /// Trilinear scaler
    trilinear: TrilinearScaler,
    /// HQ2x scaler
    hq2x: Hq2xScaler,
    /// Biadaptive scaler
    biadaptive: BiadaptiveScaler,
    /// Scaling cache
    cache: ScaleCache,
}'''

if old_struct in content:
    content = content.replace(old_struct, new_struct)
    print("Updated ScalerManager struct")
else:
    print("ScalerManager struct already updated or pattern not found")

# Fix 3: Update new() constructors
old_new1 = r'''Self {
            nearest: NearestScaler::new(),
            bilinear: BilinearScaler::new(),
            trilinear: TrilinearScaler::new(),
            hq2x: Hq2xScaler::new(),
            cache: ScaleCache::new(64),
        }'''

new_new1 = r'''Self {
            nearest: NearestScaler::new(),
            bilinear: BilinearScaler::new(),
            trilinear: TrilinearScaler::new(),
            hq2x: Hq2xScaler::new(),
            biadaptive: BiadaptiveScaler::new(),
            cache: ScaleCache::new(64),
        }'''

if old_new1 in content:
    content = content.replace(old_new1, new_new1, 1)  # Only first occurrence
    print("Updated new() constructor")
else:
    print("new() constructor already updated or pattern not found")

# Second constructor
old_new2 = r'''Self {
            nearest: NearestScaler::new(),
            bilinear: BilinearScaler::new(),
            trilinear: TrilinearScaler::new(),
            hq2x: Hq2xScaler::new(),
            cache: ScaleCache::new(capacity),
        }'''

new_new2 = r'''Self {
            nearest: NearestScaler::new(),
            bilinear: BilinearScaler::new(),
            trilinear: TrilinearScaler::new(),
            hq2x: Hq2xScaler::new(),
            biadaptive: BiadaptiveScaler::new(),
            cache: ScaleCache::new(capacity),
        }'''

if old_new2 in content:
    content = content.replace(old_new2, new_new2)
    print("Updated with_cache_capacity() constructor")
else:
    print("with_cache_capacity() constructor already updated or pattern not found")

# Fix 4: Add Biadaptive case to scale() method
old_match = r'''let result = match params.mode {
            ScaleMode::Nearest => self.nearest.scale(src, params),
            ScaleMode::Bilinear => self.bilinear.scale(src, params),
            ScaleMode::Trilinear => self.trilinear.scale(src, params),
            ScaleMode::Hq2x => self.hq2x.scale(src, params),
            ScaleMode::Step => self.nearest.scale(src, params), // Step uses nearest
        };'''

new_match = r'''let result = match params.mode {
            ScaleMode::Nearest => self.nearest.scale(src, params),
            ScaleMode::Bilinear => self.bilinear.scale(src, params),
            ScaleMode::Trilinear => self.trilinear.scale(src, params),
            ScaleMode::Hq2x => self.hq2x.scale(src, params),
            ScaleMode::Biadaptive => self.biadaptive.scale(src, params),
            ScaleMode::Step => self.nearest.scale(src, params), // Step uses nearest
        };'''

if old_match in content:
    content = content.replace(old_match, new_match)
    print("Added Biadaptive case to scale() method")
else:
    print("scale() method already updated or pattern not found")

# Write the updated content
with open('src/graphics/scaling.rs', 'w') as f:
    f.write(content)

print("\nSuccessfully updated ScalerManager with Biadaptive support")
