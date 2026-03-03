log_data = csvread(argv(){1});

% col = log_data(:, 3);
log_error = log_data(10:end, 3);

hold on;
plot(log_error);
% plot(log_data(:, 4));
% plot(log_data(:, 5));
mean = movmean(log_error, 1000);
plot(mean, "LineWidth", 2);
% stddev = movstd(log_error, 500);
% stddev_mean = movmean(stddev, 1000);
% plot(stddev_mean * 5, "LineWidth", 2);
hold off;

drawnow;
waitfor(gcf);