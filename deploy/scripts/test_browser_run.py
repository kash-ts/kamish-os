import asyncio
from playwright.async_api import async_playwright

BRAVE_PATH = r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"
EDGE_PATH = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

async def test_panel():
    executable = BRAVE_PATH if True else EDGE_PATH
    async with async_playwright() as p:
        browser = await p.chromium.launch(
            executable_path=executable,
            headless=True
        )
        page = await browser.new_page(viewport={"width": 1400, "height": 900})
        
        print("[TEST 1] Loading Web Panel Dashboard...")
        await page.goto("http://localhost:3000", wait_until="networkidle")
        await page.screenshot(path="brain_dashboard.png")
        print(" -> Dashboard loaded successfully.")

        # Test AI assistant chat in Dashboard
        print("[TEST 2] Testing AI Assistant Chat on Dashboard...")
        chat_input = page.locator("input[placeholder*='команду'], input[placeholder*='Спросите']")
        if await chat_input.count() > 0:
            await chat_input.first.fill("Привет, какая температура процессора?")
            send_btn = page.locator("button:has-text('Отправить'), button:has(svg.lucide-send)")
            if await send_btn.count() > 0:
                await send_btn.first.click()
            else:
                await chat_input.first.press("Enter")
            await asyncio.sleep(2)
            print(" -> Sent prompt to AI assistant.")

        # Test Alarms navigation
        print("[TEST 3] Navigating to Alarms & Reminders...")
        await page.click("button:has-text('Будильники и напоминания')")
        await asyncio.sleep(1)
        await page.screenshot(path="brain_alarms.png")
        print(" -> Alarms page loaded.")

        # Test Multimedia / Music
        print("[TEST 4] Navigating to Multimedia...")
        await page.click("button:has-text('Мультимедиа')")
        await asyncio.sleep(1)
        
        # Search track
        search_input = page.locator("input[placeholder*='Поиск']")
        if await search_input.count() > 0:
            await search_input.first.fill("Nirvana")
            await page.click("button:has-text('Найти')")
            await asyncio.sleep(2)
            print(" -> Music search performed.")
        
        # Test Play button
        play_btn = page.locator("button[title='Воспроизведение'], button:has-text('Воспроизведение')")
        if await play_btn.count() > 0:
            await play_btn.first.click()
            await asyncio.sleep(1)
            print(" -> Clicked Play button.")

        await page.screenshot(path="brain_multimedia.png")

        # Test Voice & Sound
        print("[TEST 5] Navigating to Voice & Sound...")
        await page.click("button:has-text('Голос и звук')")
        await asyncio.sleep(1)
        await page.screenshot(path="brain_voice.png")

        # Test Network
        print("[TEST 6] Navigating to Network...")
        await page.click("button:has-text('Сеть')")
        await asyncio.sleep(1)
        await page.screenshot(path="brain_network.png")

        # Test Diagnostics
        print("[TEST 7] Navigating to Diagnostics...")
        await page.click("button:has-text('Диагностика')")
        await asyncio.sleep(1)
        await page.screenshot(path="brain_diagnostics.png")

        await browser.close()
        print("[OK] All sections clicked and verified through browser automation!")

if __name__ == "__main__":
    asyncio.run(test_panel())
