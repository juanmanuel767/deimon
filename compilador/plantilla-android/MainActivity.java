package {{PAQUETE}};

import android.app.Activity;
import android.graphics.Color;
import android.graphics.Typeface;
import android.os.Bundle;
import android.text.InputType;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.view.WindowInsets;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.FrameLayout;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.CookieHandler;
import java.net.CookieManager;
import java.net.HttpURLConnection;
import java.net.URL;
import java.net.URLEncoder;
import java.util.ArrayList;
import java.util.List;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.net.Socket;

public class MainActivity extends Activity {
    private WebView vista;
    private FrameLayout raiz;
    private ScrollView desplazable;
    private volatile Process proceso;
    private volatile boolean cerrada = false;
    private volatile int puertoPrograma = 0;
    /** Caminos de las pantallas nativas visitadas, para el botón atrás. */
    private final List<String> historial = new ArrayList<String>();
    /** Cuadros de texto de la pantalla actual: (nombre, cuadro). */
    private final List<Object[]> cuadros = new ArrayList<Object[]>();
    private static final int MORADO = Color.rgb(124, 58, 237);

    @Override
    protected void onCreate(Bundle estado) {
        super.onCreate(estado);

        CookieHandler.setDefault(new CookieManager());
        raiz = new FrameLayout(this);
        raiz.setOnApplyWindowInsetsListener(new View.OnApplyWindowInsetsListener() {
            @Override
            public WindowInsets onApplyWindowInsets(View v, WindowInsets insets) {
                v.setPadding(insets.getSystemWindowInsetLeft(), insets.getSystemWindowInsetTop(),
                        insets.getSystemWindowInsetRight(), insets.getSystemWindowInsetBottom());
                return insets;
            }
        });

        vista = new WebView(this);
        WebSettings ajustes = vista.getSettings();
        ajustes.setJavaScriptEnabled(true);
        ajustes.setDomStorageEnabled(true);
        vista.setWebViewClient(new WebViewClient() {
            @Override
            public boolean shouldOverrideUrlLoading(WebView v, WebResourceRequest peticion) {
                return false;
            }
        });
        raiz.addView(vista, new FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT));
        setContentView(raiz);
        mostrarMensaje("Iniciando la aplicación...");

        new Thread(new Runnable() {
            @Override
            public void run() {
                arrancar();
            }
        }).start();
    }

    private void arrancar() {
        final StringBuilder registro = new StringBuilder();
        try {
            int puerto;
            ServerSocket reserva = new ServerSocket(0);
            try {
                puerto = reserva.getLocalPort();
            } finally {
                reserva.close();
            }

            File datos = new File(getFilesDir(), "datos");
            datos.mkdirs();
            File programa = new File(getApplicationInfo().nativeLibraryDir, "liblnprograma.so");

            ProcessBuilder constructor = new ProcessBuilder(programa.getAbsolutePath());
            constructor.environment().put("DEIMON_PUERTO", String.valueOf(puerto));
            constructor.environment().put("DEIMON_DATOS", datos.getAbsolutePath());
            constructor.directory(getFilesDir());
            constructor.redirectErrorStream(true);
            final Process p = constructor.start();
            proceso = p;
            if (cerrada) {
                p.destroy();
                return;
            }

            Thread lector = new Thread(new Runnable() {
                @Override
                public void run() {
                    try {
                        InputStream entrada = p.getInputStream();
                        byte[] buffer = new byte[1024];
                        int n;
                        while ((n = entrada.read(buffer)) > 0) {
                            synchronized (registro) {
                                registro.append(new String(buffer, 0, n, "UTF-8"));
                                if (registro.length() > 2000) {
                                    registro.delete(0, registro.length() - 2000);
                                }
                            }
                        }
                    } catch (Exception e) {
                        // el proceso terminó
                    }
                }
            });
            lector.setDaemon(true);
            lector.start();

            long limite = System.currentTimeMillis() + 5000;
            while (System.currentTimeMillis() < limite && !cerrada) {
                if (haTerminado(p)) {
                    break;
                }
                if (puertoAbierto(puerto)) {
                    puertoPrograma = puerto;
                    abrir("/", null, true);
                    return;
                }
                Thread.sleep(100);
            }
        } catch (Exception e) {
            synchronized (registro) {
                registro.append(e.toString());
            }
        }
        if (cerrada) {
            return;
        }
        String detalle;
        synchronized (registro) {
            detalle = registro.toString();
        }
        mostrarMensaje("No se pudo iniciar el programa de la aplicación." +
                (detalle.length() > 0 ? "\n\n" + detalle : ""));
    }

    /**
     * Pide un camino al programa. Si responde una pantalla nativa (#deimon-pantalla) la
     * dibuja con controles de Android; si es una página web, la muestra en el WebView.
     */
    private void abrir(final String camino, final String formulario, final boolean guardar) {
        new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    URL url = new URL("http://127.0.0.1:" + puertoPrograma + camino);
                    HttpURLConnection conexion = (HttpURLConnection) url.openConnection();
                    conexion.setRequestProperty("X-Deimon-App", "1");
                    if (formulario != null) {
                        conexion.setRequestMethod("POST");
                        conexion.setDoOutput(true);
                        conexion.setRequestProperty("Content-Type", "application/x-www-form-urlencoded");
                        OutputStream salida = conexion.getOutputStream();
                        salida.write(formulario.getBytes("UTF-8"));
                        salida.close();
                    }
                    InputStream entrada = conexion.getResponseCode() >= 400 ? conexion.getErrorStream() : conexion.getInputStream();
                    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
                    byte[] buffer = new byte[4096];
                    int n;
                    while (entrada != null && (n = entrada.read(buffer)) > 0) {
                        bytes.write(buffer, 0, n);
                    }
                    final String cuerpo = bytes.toString("UTF-8");
                    final String destino = conexion.getURL().getFile();
                    runOnUiThread(new Runnable() {
                        @Override
                        public void run() {
                            if (cerrada) {
                                return;
                            }
                            if (cuerpo.startsWith("#deimon-pantalla\n")) {
                                if (guardar && (historial.isEmpty() || !historial.get(historial.size() - 1).equals(destino))) {
                                    historial.add(destino);
                                }
                                dibujar(cuerpo);
                            } else {
                                raiz.removeAllViews();
                                raiz.addView(vista, new FrameLayout.LayoutParams(
                                        FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT));
                                vista.loadUrl("http://127.0.0.1:" + puertoPrograma + camino);
                            }
                        }
                    });
                } catch (Exception e) {
                    mostrarMensaje("No se pudo abrir " + camino + ": " + e);
                }
            }
        }).start();
    }

    private static String desescapar(String texto) {
        StringBuilder salida = new StringBuilder();
        for (int i = 0; i < texto.length(); i++) {
            char c = texto.charAt(i);
            if (c == '\\' && i + 1 < texto.length()) {
                char d = texto.charAt(++i);
                salida.append(d == 't' ? '\t' : d == 'n' ? '\n' : d);
            } else {
                salida.append(c);
            }
        }
        // Los textos de campo() vienen protegidos como HTML: se muestran tal cual se escribieron.
        return salida.toString().replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"")
                .replace("&#39;", "'").replace("&amp;", "&");
    }

    private int dp(int valor) {
        return (int) TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, valor, getResources().getDisplayMetrics());
    }

    /** Dibuja una pantalla nativa a partir de sus líneas: tipo<TAB>texto<TAB>texto. */
    private void dibujar(String cuerpo) {
        cuadros.clear();
        LinearLayout columna = new LinearLayout(this);
        columna.setOrientation(LinearLayout.VERTICAL);
        columna.setPadding(dp(20), dp(20), dp(20), dp(20));
        String[] lineas = cuerpo.split("\n");
        for (int i = 1; i < lineas.length; i++) {
            if (lineas[i].isEmpty()) {
                continue;
            }
            String[] partes = lineas[i].split("\t", -1);
            String tipo = partes[0];
            final String a = partes.length > 1 ? desescapar(partes[1]) : "";
            final String b = partes.length > 2 ? desescapar(partes[2]) : "";
            LinearLayout.LayoutParams margen = new LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
            margen.setMargins(0, dp(6), 0, dp(6));
            View elemento;
            if (tipo.equals("titulo")) {
                TextView t = new TextView(this);
                t.setText(a);
                t.setTextSize(26);
                t.setTypeface(Typeface.DEFAULT_BOLD);
                t.setTextColor(MORADO);
                elemento = t;
            } else if (tipo.equals("parrafo") || tipo.equals("elemento")) {
                TextView t = new TextView(this);
                t.setText(tipo.equals("elemento") ? "•  " + a : a);
                t.setTextSize(17);
                elemento = t;
            } else if (tipo.equals("separador")) {
                View linea = new View(this);
                linea.setBackgroundColor(Color.rgb(220, 220, 230));
                margen.height = dp(1);
                elemento = linea;
            } else if (tipo.equals("boton") || tipo.equals("enviar")) {
                Button boton = new Button(this);
                boton.setText(a);
                boton.setAllCaps(false);
                boton.setTextSize(17);
                boton.setTextColor(Color.WHITE);
                boton.setBackgroundColor(MORADO);
                final boolean envia = tipo.equals("enviar");
                boton.setOnClickListener(new View.OnClickListener() {
                    @Override
                    public void onClick(View v) {
                        abrir(b, envia ? formulario() : null, true);
                    }
                });
                elemento = boton;
            } else if (tipo.equals("cuadro") || tipo.equals("clave")) {
                EditText cuadro = new EditText(this);
                cuadro.setHint(b);
                cuadro.setSingleLine(true);
                if (tipo.equals("clave")) {
                    cuadro.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
                }
                cuadros.add(new Object[] {a, cuadro});
                elemento = cuadro;
            } else {
                continue;
            }
            columna.addView(elemento, margen);
        }
        desplazable = new ScrollView(this);
        desplazable.addView(columna);
        raiz.removeAllViews();
        raiz.addView(desplazable, new FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT, Gravity.TOP));
    }

    /** Los cuadros de texto de la pantalla, como un formulario (nombre=valor&...). */
    private String formulario() {
        StringBuilder datos = new StringBuilder();
        try {
            for (Object[] par : cuadros) {
                if (datos.length() > 0) {
                    datos.append('&');
                }
                datos.append(URLEncoder.encode((String) par[0], "UTF-8")).append('=')
                        .append(URLEncoder.encode(((EditText) par[1]).getText().toString(), "UTF-8"));
            }
        } catch (Exception e) {
            // UTF-8 siempre existe
        }
        return datos.toString();
    }

    private boolean haTerminado(Process p) {
        try {
            p.exitValue();
            return true;
        } catch (IllegalThreadStateException e) {
            return false;
        }
    }

    private boolean puertoAbierto(int puerto) {
        Socket socket = new Socket();
        try {
            socket.connect(new InetSocketAddress("127.0.0.1", puerto), 200);
            return true;
        } catch (Exception e) {
            return false;
        } finally {
            try {
                socket.close();
            } catch (Exception e) {
                // nada que hacer
            }
        }
    }

    private void mostrarMensaje(String texto) {
        final String html = "<html><head><meta name='viewport' content='width=device-width, initial-scale=1'></head>"
                + "<body style='font-family:sans-serif;padding:1.5em'><p style='white-space:pre-wrap'>"
                + texto.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
                + "</p></body></html>";
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                if (vista != null) {
                    vista.loadDataWithBaseURL(null, html, "text/html", "UTF-8", null);
                }
            }
        });
    }

    @Override
    public void onBackPressed() {
        if (vista != null && vista.getParent() == null && historial.size() > 1) {
            historial.remove(historial.size() - 1);
            abrir(historial.get(historial.size() - 1), null, false);
            return;
        }
        if (vista != null && vista.canGoBack()) {
            vista.goBack();
        } else {
            super.onBackPressed();
        }
    }

    @Override
    protected void onDestroy() {
        cerrada = true;
        Process p = proceso;
        if (p != null) {
            p.destroy();
        }
        if (vista != null) {
            vista.destroy();
        }
        super.onDestroy();
    }
}
