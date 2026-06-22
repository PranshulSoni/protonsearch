from PIL import Image
import os

def convert_to_ico(png_path, ico_path):
    print(f"Converting {png_path} to {ico_path}...")
    if not os.path.exists(png_path):
        print(f"  Error: {png_path} not found.")
        return False
    
    img = Image.open(png_path)
    # Save with multiple standard sizes, including 24x24 for our UI
    img.save(ico_path, format="ICO", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (256, 256)])
    print(f"  Successfully saved {ico_path}.")
    return True

def main():
    assets_dir = r"c:\Users\Pranshul Soni\Documents\Projects\Backend\Project-Raycast\assets\logo"
    settings_png = os.path.join(assets_dir, "settings.png")
    settings_ico = os.path.join(assets_dir, "settings.ico")
    control_png = os.path.join(assets_dir, "control_panel.png")
    control_ico = os.path.join(assets_dir, "control_panel.ico")
    
    convert_to_ico(settings_png, settings_ico)
    convert_to_ico(control_png, control_ico)

if __name__ == "__main__":
    main()
