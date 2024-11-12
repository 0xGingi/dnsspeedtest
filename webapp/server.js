import express from 'express';
import cors from 'cors';
import fetch from 'node-fetch';
import path from 'path';

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
                'Accept': 'application/dns-json',
                'Content-Type': 'application/dns-json',
            },
            method: 'GET',
        });
        
        if (!response.ok) {
            console.error('DNS server responded with:', response.status, response.statusText);
            const text = await response.text();
            console.error('Response body:', text);
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