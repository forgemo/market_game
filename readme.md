  
  
## api

    => GET /portfolio/<id>
    => GET /asset/<id>
    => GET /asset
    => DELETE /portfolio/<portfolio>/asset/<asset>/order/<order>
    => GET /book/<asset>
    => GET /book
    
    
    => POST /portfolio/<portfolio>/asset/<asset>/sell {"quantity":2,"mode":{"Limit":3}}
    => POST /portfolio/<portfolio>/asset/<asset>/buy {"quantity":2,"mode":"Best"}
    
    test-server: https://marketgame.cfapps.io
    
    test-portfolio: f22f799b-d56e-4f60-91d8-a3b25dae61a4
    test-portfolio: ddf7e30f-3987-436b-acfe-ac4c7b8994de
    
   
 Examples: 
 
https://marketgame.cfapps.io/book
 
https://marketgame.cfapps.io/portfolio/f22f799b-d56e-4f60-91d8-a3b25dae61a4

https://marketgame.cfapps.io/asset

https://marketgame.cfapps.io/asset/3b936534-fc8d-42b8-adfa-58e06023d6cf
