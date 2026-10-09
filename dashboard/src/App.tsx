import { useEffect, useState } from 'react'
import Button from '@mui/material/Button'
import { AppBar, createTheme, CssBaseline, IconButton, ThemeProvider, Toolbar, Typography } from '@mui/material';
import Brightness4Icon from '@mui/icons-material/Brightness4';
import { useQuery } from '@tanstack/react-query';

function usePolling() {
  return useQuery({
    queryKey: ['dashboard'],

  })
}

function App() {
  const [count, setCount] = useState(0)
  const [dark, setDark] = useState(() => {
    return localStorage.getItem('theme') === 'dark'
  })
  useEffect(() => {
    localStorage.setItem('theme', dark ? 'dark' : 'light')  // 写
  }, [dark])

  const theme = createTheme({
    palette: {
      mode: dark ? 'dark' : 'light',
      primary: { main: dark ? '#9800ff' : '#4c009e' },
    },
    shape: { borderRadius: 4 },
    spacing: 8,
  })

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />

      <AppBar position="fixed" sx={{ zIndex: (t) => t.zIndex.drawer + 1 }}>
        <Toolbar>
          <Typography variant="h6" sx={{ flexGrow: 1 }}>
            cuscuta - status
          </Typography>
          <IconButton color="inherit" onClick={() => {
            setDark(!dark);
          }}>{/* 暗色切换按钮放这 */}<Brightness4Icon /></IconButton>
        </Toolbar>
      </AppBar>

    </ThemeProvider>
  )
}

export default App
