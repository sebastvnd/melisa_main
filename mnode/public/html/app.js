/**
 * MNode Custom App JavaScript
 * Example client-side script
 */

document.addEventListener('DOMContentLoaded', async () => {
    console.log('MNode App loaded!');

    // Fetch node info
    try {
        const response = await fetch('/api/info');
        const data = await response.json();
        console.log('Node Info:', data);

        // You can use this data to customize page
        // For example:
        // document.title = `${data.route_path} - MNode`;
    } catch (error) {
        console.error('Failed to fetch node info:', error);
    }

    // Example: Health check
    setInterval(async () => {
        try {
            const response = await fetch('/api/health');
            const data = await response.json();
            if (data.status === 'healthy') {
                console.log('Node is healthy ✓');
            }
        } catch (error) {
            console.error('Health check failed:', error);
        }
    }, 30000); // Check every 30 seconds
});

/**
 * Utility functions untuk Melisa integration
 */

// Get node information
async function getNodeInfo() {
    try {
        const response = await fetch('/api/info');
        return await response.json();
    } catch (error) {
        console.error('Failed to get node info:', error);
        return null;
    }
}

// Check node health
async function checkHealth() {
    try {
        const response = await fetch('/api/health');
        const data = await response.json();
        return data.status === 'healthy';
    } catch (error) {
        console.error('Health check failed:', error);
        return false;
    }
}

// Log to console dengan prefix
function log(message, type = 'log') {
    const timestamp = new Date().toISOString().substring(11, 19);
    console[type](`[${timestamp}] [MNode] ${message}`);
}

log('App initialized');
