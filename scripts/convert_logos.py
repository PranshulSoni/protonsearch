from PIL import Image, ImageDraw
import os

def generate_search_png(png_path):
    print(f"Generating search icon PNG at {png_path}...")
    # Create 256x256 image with transparent background
    img = Image.new("RGBA", (256, 256), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Draw magnifying glass circle (outline color matches CLR_GRAY: 148, 148, 154)
    # Circle bounds: [45, 45, 175, 175] (center 110, radius 65)
    draw.ellipse([45, 45, 175, 175], outline=(148, 148, 154, 255), width=18)
    
    # Draw handle starting from circle boundary down-right
    # 110 + 65 * cos(45 deg) = 156
    draw.line([156, 156, 215, 215], fill=(148, 148, 154, 255), width=18)
    
    img.save(png_path)

def convert_to_ico(png_path, ico_path):
    print(f"Converting {png_path} to {ico_path} with square padding...")
    if not os.path.exists(png_path):
        print(f"  Error: {png_path} not found.")
        return False
    
    img = Image.open(png_path).convert("RGBA")
    w, h = img.size
    max_dim = max(w, h)
    
    # Create square canvas with transparency
    square_img = Image.new("RGBA", (max_dim, max_dim), (0, 0, 0, 0))
    # Center the original image
    square_img.paste(img, ((max_dim - w) // 2, (max_dim - h) // 2))
    
    # Save with multiple standard sizes, including 24x24 for our UI
    square_img.save(ico_path, format="ICO", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (256, 256)])
    print(f"  Successfully saved {ico_path}.")
    return True

def main():
    assets_dir = r"c:\Users\Pranshul Soni\Documents\Projects\Backend\Project-Raycast\assets\logo"
    settings_png = os.path.join(assets_dir, "settings.png")
    settings_ico = os.path.join(assets_dir, "settings.ico")
    control_png = os.path.join(assets_dir, "control_panel.png")
    control_ico = os.path.join(assets_dir, "control_panel.ico")
    search_png = os.path.join(assets_dir, "search.png")
    search_ico = os.path.join(assets_dir, "search.ico")
    
    convert_to_ico(settings_png, settings_ico)
    convert_to_ico(control_png, control_ico)
    
    generate_search_png(search_png)
    convert_to_ico(search_png, search_ico)

if __name__ == "__main__":
    main()
