let canvas = null;
let ctx = null;
let stars = [];
let animFrame = null;
let running = false;
function resizeCanvas() {
    if (!canvas)
        return;
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
}
function generateStars() {
    if (!canvas)
        return;
    stars = [];
    const count = Math.floor((canvas.width * canvas.height) / 2800);
    for (let i = 0; i < count; i++) {
        const bright = Math.random() < 0.08;
        stars.push({
            x: Math.random() * canvas.width,
            y: Math.random() * canvas.height,
            r: bright ? Math.random() * 1.4 + 0.4 : Math.random() * 0.7 + 0.2,
            opacity: Math.random() * 0.7 + 0.3,
            speed: bright ? Math.random() * 0.003 + 0.001 : Math.random() * 0.008 + 0.004,
        });
    }
}
function drawStars() {
    if (!ctx || !canvas || !running) {
        animFrame = requestAnimationFrame(drawStars);
        return;
    }
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    const now = Date.now();
    for (const s of stars) {
        const pulse = s.opacity + Math.sin(now * s.speed) * 0.15;
        ctx.beginPath();
        ctx.arc(s.x, s.y, s.r, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(200, 220, 255, ${Math.max(0.1, Math.min(1, pulse))})`;
        ctx.fill();
    }
    animFrame = requestAnimationFrame(drawStars);
}
function start() {
    if (running)
        return;
    running = true;
    resizeCanvas();
    generateStars();
    drawStars();
}
function stop() {
    running = false;
    if (animFrame !== null) {
        cancelAnimationFrame(animFrame);
        animFrame = null;
    }
    if (ctx && canvas) {
        ctx.clearRect(0, 0, canvas.width, canvas.height);
    }
}
function onThemeChange(theme) {
    if (!canvas)
        return;
    if (theme === 'mygo') {
        canvas.style.display = 'block';
        start();
    }
    else {
        canvas.style.display = 'none';
        stop();
    }
}
export function initStarfield() {
    canvas = document.getElementById('mygo-stars');
    if (!canvas)
        return;
    ctx = canvas.getContext('2d');
    if (!ctx)
        return;
    window.addEventListener('resize', () => {
        resizeCanvas();
        generateStars();
    });
    const observer = new MutationObserver(() => {
        const theme = document.body.getAttribute('data-theme') || 'dark';
        onThemeChange(theme);
    });
    observer.observe(document.body, { attributes: true, attributeFilter: ['data-theme'] });
    const initialTheme = document.body.getAttribute('data-theme') || 'dark';
    onThemeChange(initialTheme);
}
//# sourceMappingURL=starfield.js.map