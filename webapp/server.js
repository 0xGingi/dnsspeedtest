const express = require('express');
const cors = require('cors');
const fetch = require('node-fetch');
const path = require('path');
const app = express();

app.use(cors());
app.use(express.static('.'));

app.get('/proxy', async (req, res) => {
    const { url } = req.query;
    if (!url) {
        return res.status(400).send('URL parameter is required');
    }

    console.log('Proxying request to:', url);

    try {
        const response = await fetch(url, {
            headers: {
                'accept': 'application/dns-json',
                'content-type': 'application/dns-json',
            }
        });
        
        if (!response.ok) {
            console.error('DNS server responded with:', response.status, response.statusText);
            return res.status(response.status).json({ 
                error: `DNS server responded with: ${response.status} ${response.statusText}` 
            });
        }

        const data = await response.json();
        res.json(data);
    } catch (error) {
        console.error('Proxy error:', error);
        res.status(500).json({ error: error.message });
    }
});

const PORT = process.env.PORT || 3200;
app.listen(PORT, () => {
    console.log(`Server running on port ${PORT}`);
});