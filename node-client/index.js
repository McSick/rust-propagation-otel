
const express = require('express');
const app = express();
const http = require('http');
const port = 3000;

// call to localhost:8080/rolldice using http module

app.get('/', (req, res) => {
    res.send('Hello World!');
}
);


app.get('/rolldice', (req, res) => {
    const options = {
        hostname: '127.0.0.1',
        port: 8080,
        path: '/rolldice',
        method: 'GET'
    };

    const rustcall = http.request(options, (response) => {
        console.log('statusCode:', response.statusCode);
        console.log('headers:', response.headers);

        response.on('data', (d) => {
            res.send({data: d});
        });
    });

    rustcall.on('error', (e) => {
        console.error(e);
    });

    rustcall.end();
}
);

app.listen(port, () => {
    console.log(`Example app listening at http://localhost:${port}`);
}
);
